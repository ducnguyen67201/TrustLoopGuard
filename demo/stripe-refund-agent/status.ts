import type {
  Client,
  FinancialActionRecord,
  FinancialReceipt,
} from '@trustloopguard/sdk';

import { DEMO_ORDER_ID, REFUND_AGENT_ID } from './types';

export type RefundDemoStatusClient = Pick<
  Client,
  'getFinancialAction' | 'getFinancialDecisionReceipt' | 'getReceipt'
>;

export interface RefundDemoActionStatus {
  actionId: string;
  status: FinancialActionRecord['status'];
  orderId: string;
  amountMinor: number;
  currency: 'USD';
  receiptId?: string;
  providerReference?: string;
  updatedAt: string;
}

export async function readRefundDemoActionStatus(
  client: RefundDemoStatusClient,
  actionId: string,
): Promise<RefundDemoActionStatus> {
  const action = await client.getFinancialAction(actionId);
  const metadata = action.action.metadata;
  const orderId = stringMetadata(metadata, 'order_id');
  const demoRequestId = stringMetadata(metadata, 'demo_request_id');
  if (
    action.id !== actionId ||
    action.action.kind !== 'refund' ||
    action.action.principal_id !== REFUND_AGENT_ID ||
    orderId !== DEMO_ORDER_ID ||
    demoRequestId === undefined
  ) {
    throw new Error('refund demo action not found');
  }

  const amountMinor = Number(action.action.amount.amount_minor);
  if (!Number.isSafeInteger(amountMinor) || amountMinor <= 0) {
    throw new Error('refund demo action amount is invalid');
  }

  let receiptId: string | undefined;
  let providerReference: string | undefined;
  let updatedAt = action.updated_at;
  if (action.status === 'executed') {
    const decision = await client.getFinancialDecisionReceipt(actionId);
    receiptId = decision.execution.receipt_id;
    if (receiptId !== undefined) {
      const receipt = await client.getReceipt(receiptId);
      providerReference = providerReferenceFromReceipt(receipt);
      updatedAt = receipt.created_at;
    }
  }

  return {
    actionId,
    status: action.status,
    orderId,
    amountMinor,
    currency: 'USD',
    receiptId,
    providerReference,
    updatedAt,
  };
}

function stringMetadata(
  metadata: Record<string, unknown> | null,
  key: string,
): string | undefined {
  const value = metadata?.[key];
  return typeof value === 'string' && value !== '' ? value : undefined;
}

function providerReferenceFromReceipt(receipt: FinancialReceipt): string | undefined {
  const provider = receipt.proof?.['provider'];
  if (typeof provider !== 'object' || provider === null || !('reference' in provider)) {
    return undefined;
  }
  return typeof provider.reference === 'string' ? provider.reference : undefined;
}
