import type {
  CreateFinancialActionRequest,
  FinancialActionRecord,
  FinancialActionStatus,
  FinancialMandate,
  FinancialMandateListResponse,
  FinancialReceipt,
} from '@trustloopguard/sdk';

import { searchOrder } from './orders';
import {
  DEMO_PAYMENT_METHOD_ID,
  REFUND_AGENT_ID,
  REFUND_MANDATE_ID,
  REFUND_MANDATE_VERSION,
  type ExecuteRefundResult,
  type OrderSearchQuery,
  type OrderSearchResult,
  type PrepareRefundInput,
  type PrepareRefundResult,
} from './types';

export interface RefundAgentClient {
  createMandate(req: {
    id: string;
    version: number;
    principal_id: string;
    scope: Record<string, string | number | string[]>;
    metadata: Record<string, string>;
  }): Promise<FinancialMandate>;
  listMandates(): Promise<FinancialMandateListResponse>;
  guardPayment(req: CreateFinancialActionRequest): Promise<FinancialActionRecord>;
  getFinancialAction(actionId: string): Promise<FinancialActionRecord>;
  executeAction(actionId: string): Promise<FinancialActionRecord>;
  getReceipt(receiptId: string): Promise<FinancialReceipt>;
}

export const REFUND_MANDATE_SCOPE = {
  action_kinds: ['refund'],
  rails: ['payment_http'],
  max_amount_minor: 10_000,
  currency: 'USD',
};

export async function ensureRefundMandate(client: RefundAgentClient): Promise<FinancialMandate> {
  const existing = await client.listMandates();
  const active = existing.mandates.find(
    (mandate) =>
      mandate.id === REFUND_MANDATE_ID &&
      mandate.version === REFUND_MANDATE_VERSION &&
      mandate.status === 'active',
  );
  if (active !== undefined) return active;

  return client.createMandate({
    id: REFUND_MANDATE_ID,
    version: REFUND_MANDATE_VERSION,
    principal_id: REFUND_AGENT_ID,
    scope: REFUND_MANDATE_SCOPE,
    metadata: {
      source: 'stripe_refund_agent_demo',
      workflow: 'support_refund',
    },
  });
}

export function searchOrderTool(query: OrderSearchQuery): OrderSearchResult {
  return searchOrder(query);
}

export async function prepareRefundTool(
  input: PrepareRefundInput,
  client: RefundAgentClient,
): Promise<PrepareRefundResult> {
  if (!Number.isInteger(input.amountMinor) || input.amountMinor <= 0) {
    throw new Error('refund amount must be a positive integer minor-unit amount');
  }

  const search = searchOrderTool({ orderId: input.orderId });
  if (!search.found || search.order === undefined) {
    throw new Error(`cannot prepare refund: ${search.reason ?? 'order not found'}`);
  }
  if (input.amountMinor > search.order.refundableBalanceMinor) {
    throw new Error(
      `cannot prepare refund: requested ${formatMoney(input.amountMinor)} but only ${formatMoney(
        search.order.refundableBalanceMinor,
      )} is refundable`,
    );
  }

  await ensureRefundMandate(client);
  const request = buildRefundActionRequest(input, search);
  const action = await client.guardPayment(request);
  return {
    action,
    request,
    order: search.order,
    status: action.status,
    message: messageForStatus(action.status, action.id),
  };
}

export async function executeRefundTool(
  actionId: string,
  client: RefundAgentClient,
): Promise<ExecuteRefundResult> {
  const current = await client.getFinancialAction(actionId);
  if (current.status === 'held') {
    return {
      action: current,
      status: current.status,
      message: `refund ${actionId} is held for approval; no Stripe refund was created`,
    };
  }
  if (current.status === 'denied' || current.status === 'failed' || current.status === 'expired') {
    return {
      action: current,
      status: current.status,
      message: `refund ${actionId} is ${current.status}; no Stripe refund was created`,
    };
  }

  const executed = current.status === 'executed' ? current : await client.executeAction(actionId);
  const receipt = executed.status === 'executed' ? await client.getReceipt(executed.id) : undefined;
  return {
    action: executed,
    receipt,
    status: executed.status,
    message:
      executed.status === 'executed'
        ? `refund ${executed.id} executed through TrustLoopGuard`
        : `refund ${executed.id} is ${executed.status}; no Stripe refund was created`,
  };
}

export function buildRefundActionRequest(
  input: PrepareRefundInput,
  search: OrderSearchResult,
): CreateFinancialActionRequest {
  if (!search.found || search.order === undefined) {
    throw new Error('cannot build refund action without a found order');
  }

  const reason = normalizeReason(input.reason);
  return {
    idempotency_key: `stripe-refund-agent:${search.order.id}:${input.amountMinor}:${reason}`,
    execute: false,
    action: {
      kind: 'refund',
      principal_id: REFUND_AGENT_ID,
      amount: {
        amount_minor: BigInt(input.amountMinor),
        currency: search.order.currency,
      },
      counterparty: {
        id: search.order.customerId,
        display_name: search.order.customerName,
        kind: 'customer',
        country: 'US',
        metadata: {
          customer_email: search.order.customerEmail,
          original_payment_method_id: search.order.paymentMethodId,
        },
      },
      rail: 'payment_http',
      mandate: {
        id: REFUND_MANDATE_ID,
        version: REFUND_MANDATE_VERSION,
      },
      memo: `Refund ${search.order.id}: ${reason}`,
      metadata: {
        order_id: search.order.id,
        customer_id: search.order.customerId,
        payment_intent_id: search.order.paymentIntentId,
        destination_payment_method_id: DEMO_PAYMENT_METHOD_ID,
        reason,
      },
    },
    evidence: [
      {
        source: 'customer_backend',
        source_id: search.evidenceRef,
        kind: 'refund_eligibility',
        observed_at: '2026-07-06T10:00:00.000Z',
        metadata: {
          order_exists: search.evidence.orderExists,
          payment_captured: search.evidence.paymentCaptured,
          refundable_balance_minor: search.order.refundableBalanceMinor,
          refund_window_open: search.evidence.refundWindowOpen,
          amount_lte_refundable_balance: input.amountMinor <= search.order.refundableBalanceMinor,
          destination_is_original_payment_method:
            search.evidence.destinationIsOriginalPaymentMethod,
          no_duplicate_refund: search.evidence.noDuplicateRefund,
        },
      },
    ],
  };
}

export function formatMoney(amountMinor: number): string {
  return `$${(amountMinor / 100).toFixed(2)}`;
}

function normalizeReason(reason: string): string {
  const clean = reason.trim().toLowerCase().replaceAll(/[^a-z0-9]+/g, '_').replaceAll(/^_|_$/g, '');
  return clean || 'customer_request';
}

function messageForStatus(status: FinancialActionStatus, actionId: string): string {
  if (status === 'authorized') return `refund ${actionId} authorized`;
  if (status === 'held') return `refund ${actionId} requires approval`;
  if (status === 'denied') return `refund ${actionId} denied`;
  return `refund ${actionId} is ${status}`;
}
