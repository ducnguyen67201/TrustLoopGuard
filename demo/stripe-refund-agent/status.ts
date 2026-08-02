import type {
  AuthorizationApproval,
  Client,
  FinancialActionRecord,
  FinancialReceipt,
} from '@featherlane-ai/sdk';

import { DEMO_ORDER_ID, REFUND_AGENT_ID } from './types';

export type RefundDemoStatusClient = Pick<
  Client,
  'executeAction' | 'getApproval' | 'getAuthorizationReceipt' | 'getFinancialAction' | 'getReceipt'
>;
type ExecuteFinancialActionRequest = NonNullable<Parameters<Client['executeAction']>[1]>;

export interface RefundDemoActionStatus {
  actionId: string;
  authorizationEffect: FinancialActionRecord['authorization_effect'];
  executionStatus: FinancialActionRecord['execution_status'];
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
  const action = await maybeExecuteApprovedAction(client, await client.getFinancialAction(actionId));
  return statusFromAction(client, actionId, action);
}

async function maybeExecuteApprovedAction(
  client: RefundDemoStatusClient,
  action: FinancialActionRecord,
): Promise<FinancialActionRecord> {
  if (
    action.authorization_effect !== 'require_approval' ||
    action.execution_status !== 'not_started' ||
    action.authorization_receipt_id === undefined
  ) {
    return action;
  }

  const grantId = await approvedGrantIdForAction(client, action);
  if (grantId === undefined) return action;

  const attemptId = `stripe-refund-agent:execute:${action.id}`;
  const request: ExecuteFinancialActionRequest = {
    authorization: { grant_id: grantId, attempt_id: attemptId },
    attempt_id: attemptId,
  };
  return client.executeAction(action.id, request);
}

async function approvedGrantIdForAction(
  client: RefundDemoStatusClient,
  action: FinancialActionRecord,
): Promise<string | undefined> {
  try {
    const receipt = await client.getAuthorizationReceipt(action.authorization_receipt_id!);
    if (receipt.approval_id === undefined) return undefined;
    const approval = await client.getApproval(receipt.approval_id);
    if (!approvalBelongsToAction(approval, action)) return undefined;
    return approval.status === 'approved' ? approval.grant_id : undefined;
  } catch {
    return undefined;
  }
}

function approvalBelongsToAction(
  approval: AuthorizationApproval,
  action: FinancialActionRecord,
): boolean {
  return (
    approval.workspace_id === action.workspace_id &&
    approval.environment_id === action.environment_id &&
    approval.envelope.domain === 'financial' &&
    approval.envelope.principal_id === REFUND_AGENT_ID &&
    approval.envelope.subject_id === action.id
  );
}

async function statusFromAction(
  client: Pick<RefundDemoStatusClient, 'getReceipt'>,
  actionId: string,
  action: FinancialActionRecord,
): Promise<RefundDemoActionStatus> {
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
  if (action.execution_status === 'succeeded') {
    const receipt = await client.getReceipt(actionId);
    receiptId = receipt.id;
    providerReference = providerReferenceFromReceipt(receipt);
    updatedAt = receipt.created_at;
  }

  return {
    actionId,
    authorizationEffect: action.authorization_effect,
    executionStatus: action.execution_status,
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
