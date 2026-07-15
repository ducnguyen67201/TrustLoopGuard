import type {
  AuthorizationEffect,
  AuthorizationGrant,
  AuthorizationGrantListResponse,
  CreateAuthorizationGrantRequest,
  CreateFinancialActionRequest,
  FinancialOperation,
  FinancialOperationSpec,
  FinancialActionRecord,
  FinancialExecutionStatus,
  FinancialReceipt,
} from '@trustloopguard/sdk';

import { searchOrder } from './orders';
import { recordRefundExecution } from './order-db';
import {
  DEMO_PAYMENT_METHOD_ID,
  REFUND_AGENT_ID,
  REFUND_GRANT_CAPABILITY,
  REFUND_GRANT_REQUIREMENT_IDS,
  type ExecuteRefundResult,
  type OrderSearchQuery,
  type OrderSearchResult,
  type PrepareRefundInput,
  type PrepareRefundResult,
} from './types';

export interface RefundAgentClient {
  createGrant(req: CreateAuthorizationGrantRequest): Promise<AuthorizationGrant>;
  listGrants(): Promise<AuthorizationGrantListResponse>;
  financialOperation<Input, Facts>(
    spec: FinancialOperationSpec<Input, Facts>,
  ): FinancialOperation<Input, Facts>;
  guardPayment(req: CreateFinancialActionRequest): Promise<FinancialActionRecord>;
  getFinancialAction(actionId: string): Promise<FinancialActionRecord>;
  executeAction(
    actionId: string,
    request: { authorization: { grant_id: string; attempt_id: string }; attempt_id: string },
  ): Promise<FinancialActionRecord>;
  getReceipt(receiptId: string): Promise<FinancialReceipt>;
}

export interface RefundAuthorizationOptions {
  grantId?: string;
  allowGrantProvisioning?: boolean;
}

export async function ensureRefundGrant(client: RefundAgentClient): Promise<AuthorizationGrant> {
  const existing = await client.listGrants();
  const active = existing.grants.find(
    (grant) =>
      grant.principal_id === REFUND_AGENT_ID &&
      grant.capability === REFUND_GRANT_CAPABILITY &&
      grant.status === 'active',
  );
  if (active !== undefined) return active;

  return client.createGrant({
    principal_id: REFUND_AGENT_ID,
    domain: 'financial',
    capability: REFUND_GRANT_CAPABILITY,
    requirement_ids: REFUND_GRANT_REQUIREMENT_IDS,
    scope: {
      scope_type: 'financial',
      scope: {
        action_kinds: ['refund'],
        operation: 'issue_refund',
        rail: 'payment_http',
        currency: 'USD',
        maximum_amount_minor: 10_000n,
        counterparties: [],
        x402_hosts: [],
        x402_resources: [],
        x402_networks: [],
        x402_assets: [],
        x402_payees: [],
        required_preconditions: [],
      },
    },
  });
}

export function searchOrderTool(query: OrderSearchQuery, dbPath?: string): OrderSearchResult {
  return searchOrder(query, dbPath);
}

export async function prepareRefundTool(
  input: PrepareRefundInput,
  client: RefundAgentClient,
  dbPath?: string,
  authorizationOptions: RefundAuthorizationOptions = {},
): Promise<PrepareRefundResult> {
  if (!Number.isInteger(input.amountMinor) || input.amountMinor <= 0) {
    throw new Error('refund amount must be a positive integer minor-unit amount');
  }

  const search = searchOrderTool({ orderId: input.orderId }, dbPath);
  if (!search.found || search.order === undefined) {
    throw new Error(`cannot prepare refund: ${search.reason ?? 'order not found'}`);
  }

  const grantId = await resolveRefundGrantId(client, authorizationOptions);
  const operation = refundOperation(client, grantId);
  const request = operation.buildRequest(input, search);
  const action = await operation.verify(input, search);
  return {
    action,
    request,
    order: search.order,
    status: action.authorization_effect,
    message: messageForStatus(action.authorization_effect, action.execution_status, action.id),
  };
}

export async function executeRefundTool(
  actionId: string,
  client: RefundAgentClient,
  dbPath?: string,
  authorizationOptions: RefundAuthorizationOptions = {},
): Promise<ExecuteRefundResult> {
  let current = await client.getFinancialAction(actionId);
  if (current.authorization_effect === 'require_approval') {
    return {
      action: current,
      status: current.authorization_effect,
      message: `refund ${actionId} is held for approval; no Stripe refund was created`,
    };
  }
  if (current.authorization_effect !== 'permit') {
    return {
      action: current,
      status: current.authorization_effect,
      message: `refund ${actionId} is ${current.authorization_effect}; no Stripe refund was created`,
    };
  }

  const grantId = await resolveRefundGrantId(client, authorizationOptions);
  const attemptId = `stripe-refund-agent:execute:${actionId}`;
  const executed = current.execution_status === 'succeeded'
    ? current
    : await client.executeAction(actionId, {
        authorization: { grant_id: grantId, attempt_id: attemptId },
        attempt_id: attemptId,
      });
  const receipt = executed.execution_status === 'succeeded' ? await client.getReceipt(executed.id) : undefined;
  if (executed.execution_status === 'succeeded') {
    recordRefundExecution(
      {
        orderId: stringMetadata(executed.action.metadata, 'order_id') ?? 'unknown_order',
        financialActionId: executed.id,
        amountMinor: Number(executed.action.amount.amount_minor),
        providerReference: providerReferenceFromReceipt(receipt),
        status: 'succeeded',
        reason: stringMetadata(executed.action.metadata, 'reason') ?? 'customer_request',
      },
      dbPath,
    );
  }
  return {
    action: executed,
    receipt,
    status: executed.execution_status,
    message:
      executed.execution_status === 'succeeded'
        ? `refund ${executed.id} executed through TrustLoopGuard`
        : `refund ${executed.id} is ${executed.execution_status}; no Stripe refund was created`,
  };
}

async function resolveRefundGrantId(
  client: RefundAgentClient,
  options: RefundAuthorizationOptions,
): Promise<string> {
  const configured = cleanGrantId(options.grantId);
  if (configured !== undefined) return configured;
  if (options.allowGrantProvisioning === false) {
    throw new Error('TL_REFUND_GRANT_ID is required for the public refund runtime');
  }
  return (await ensureRefundGrant(client)).id;
}

function cleanGrantId(grantId: string | undefined): string | undefined {
  const trimmed = grantId?.trim();
  return trimmed === undefined || trimmed === '' ? undefined : trimmed;
}

export function buildRefundActionRequest(
  input: PrepareRefundInput,
  search: OrderSearchResult,
  client: Pick<RefundAgentClient, 'financialOperation'>,
): CreateFinancialActionRequest {
  if (!search.found || search.order === undefined) {
    throw new Error('cannot build refund action without a found order');
  }

  return refundOperation(client).buildRequest(input, search);
}

function refundOperation(
  client: Pick<RefundAgentClient, 'financialOperation'>,
  grantId?: string,
): FinancialOperation<PrepareRefundInput, OrderSearchResult> {
  return client.financialOperation<PrepareRefundInput, OrderSearchResult>({
    operation: 'issue_refund',
    kind: 'refund',
    principalId: REFUND_AGENT_ID,
    rail: 'payment_http',
    amount: (input, search) => {
      if (!search.found || search.order === undefined) {
        throw new Error('cannot build refund action without a found order');
      }
      return {
        amount_minor: BigInt(input.amountMinor),
        currency: search.order.currency,
      };
    },
    idempotencyKey: (input, search) => {
      if (!search.found || search.order === undefined) {
        throw new Error('cannot build refund action without a found order');
      }
      const reason = normalizeReason(input.reason);
      const baseIdempotencyKey = `stripe-refund-agent:${search.order.id}:${input.amountMinor}:${reason}`;
      return input.requestId === undefined
        ? baseIdempotencyKey
        : `${baseIdempotencyKey}:${normalizeRequestId(input.requestId)}`;
    },
    counterparty: (_input, search) => {
      if (!search.found || search.order === undefined) {
        throw new Error('cannot build refund action without a found order');
      }
      return {
        id: search.order.customerId,
        display_name: search.order.customerName,
        kind: 'customer',
        country: 'US',
        metadata: {
          customer_email: search.order.customerEmail,
          original_payment_method_id: search.order.paymentMethodId,
        },
      };
    },
    authorization: (input) => grantId === undefined ? undefined : ({
      grant_id: grantId,
      attempt_id: `stripe-refund-agent:prepare:${input.requestId ?? input.orderId}`,
    }),
    memo: (input, search) => {
      if (!search.found || search.order === undefined) {
        throw new Error('cannot build refund action without a found order');
      }
      return `Refund ${search.order.id}: ${normalizeReason(input.reason)}`;
    },
    metadata: (input, search) => {
      if (!search.found || search.order === undefined) {
        throw new Error('cannot build refund action without a found order');
      }
      return {
        order_id: search.order.id,
        customer_id: search.order.customerId,
        payment_intent_id: search.order.paymentIntentId,
        destination_payment_method_id: DEMO_PAYMENT_METHOD_ID,
        reason: normalizeReason(input.reason),
        ...(input.requestId === undefined ? {} : { demo_request_id: input.requestId }),
      };
    },
    evidence: (input, search) => {
      if (!search.found || search.order === undefined) {
        throw new Error('cannot build refund action without a found order');
      }
      return [
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
      ];
    },
  });
}

export function formatMoney(amountMinor: number): string {
  return `$${(amountMinor / 100).toFixed(2)}`;
}

function normalizeReason(reason: string): string {
  const clean = reason.trim().toLowerCase().replaceAll(/[^a-z0-9]+/g, '_').replaceAll(/^_|_$/g, '');
  return clean || 'customer_request';
}

function normalizeRequestId(requestId: string): string {
  const clean = requestId.trim().toLowerCase().replaceAll(/[^a-z0-9_-]+/g, '_');
  return clean || 'manual';
}

function messageForStatus(
  effect: AuthorizationEffect,
  execution: FinancialExecutionStatus,
  actionId: string,
): string {
  if (effect === 'permit') return `refund ${actionId} authorized; execution is ${execution}`;
  if (effect === 'require_approval') return `refund ${actionId} requires approval`;
  return `refund ${actionId} authorization is ${effect}`;
}

function stringMetadata(metadata: Record<string, unknown> | null, key: string): string | undefined {
  const value = metadata?.[key];
  return typeof value === 'string' ? value : undefined;
}

function providerReferenceFromReceipt(receipt: FinancialReceipt | undefined): string | undefined {
  const provider = receipt?.proof?.provider;
  if (typeof provider === 'object' && provider !== null && 'reference' in provider) {
    return typeof provider.reference === 'string' ? provider.reference : undefined;
  }
  const legacyValue = receipt?.proof?.provider_reference;
  return typeof legacyValue === 'string' ? legacyValue : undefined;
}
