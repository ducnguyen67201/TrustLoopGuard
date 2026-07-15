import type {
  AuthorizationEffect,
  AuthorizationGrant,
  AuthorizationReceipt,
  CreateAuthorizationGrantRequest,
  CreateFinancialActionRequest,
  FinancialActionOutcome,
  FinancialActionRecord,
  FinancialExecutionStatus,
  FinancialOutcomeListResponse,
  FinancialReceipt,
} from '@trustloopguard/sdk';

export interface FinancialDemoClient {
  createGrant(req: CreateAuthorizationGrantRequest): Promise<AuthorizationGrant>;
  guardPayment(req: CreateFinancialActionRequest): Promise<FinancialActionRecord>;
  executeAction(
    actionId: string,
    request: { authorization: { grant_id: string; attempt_id: string }; attempt_id: string },
  ): Promise<FinancialActionRecord>;
  getAuthorizationReceipt(receiptId: string): Promise<AuthorizationReceipt>;
  getReceipt(receiptId: string): Promise<FinancialReceipt>;
  recordActionOutcome(actionId: string, outcome: FinancialActionOutcome): Promise<FinancialActionOutcome>;
  listActionOutcomes(actionId: string): Promise<FinancialOutcomeListResponse>;
}

export type ScenarioKey = 'saved_grant' | 'approval_threshold' | 'duplicate_idempotency' | 'missing_grant';

export interface RefundScenario {
  key: ScenarioKey;
  label: string;
  amountMinor: bigint;
  useGrant: boolean;
  duplicateSubmit?: boolean;
}

export interface ScenarioResult {
  key: ScenarioKey;
  label: string;
  authorizationEffect: AuthorizationEffect;
  executionStatus: FinancialExecutionStatus;
  actionId: string;
  providerCalls: number;
  authorizationReceiptExported: boolean;
  executionReceiptExported: boolean;
  outcomeRecorded: boolean;
  duplicateReusedAction: boolean;
}

const PRINCIPAL_ID = 'refund-bot';
const CUSTOMER_ID = 'cust_456';
const GRANT_REQUIREMENTS = [
  'financial:refund-controls:grant_required',
  'financial:refund-controls:approval_threshold',
];

export const REFUND_SCENARIOS: RefundScenario[] = [
  { key: 'saved_grant', label: 'saved grant authorizes a $40 refund', amountMinor: 4_000n, useGrant: true },
  { key: 'approval_threshold', label: 'same bounded grant covers the $75 approval requirement', amountMinor: 7_500n, useGrant: true },
  { key: 'duplicate_idempotency', label: 'same attempt retry executes only once', amountMinor: 3_500n, useGrant: true, duplicateSubmit: true },
  { key: 'missing_grant', label: 'no grant waits in the unified approval queue', amountMinor: 3_000n, useGrant: false },
];

export async function createRefundGrant(client: FinancialDemoClient): Promise<AuthorizationGrant> {
  return client.createGrant({
    principal_id: PRINCIPAL_ID,
    domain: 'financial',
    capability: 'financial:issue_refund',
    requirement_ids: GRANT_REQUIREMENTS,
    scope: {
      scope_type: 'financial',
      scope: {
        action_kinds: ['refund'],
        operation: 'issue_refund',
        rail: 'payment_http',
        currency: 'USD',
        maximum_amount_minor: 10_000n,
        counterparties: [CUSTOMER_ID],
        x402_hosts: [], x402_resources: [], x402_networks: [], x402_assets: [], x402_payees: [],
        required_preconditions: [],
      },
    },
  });
}

export function buildRefundRequest(
  scenario: RefundScenario,
  grant: AuthorizationGrant | null,
): CreateFinancialActionRequest {
  const attemptId = `financial-refund:${scenario.key}:attempt`;
  return {
    idempotency_key: `financial-refund:${scenario.key}`,
    execute: false,
    ...(scenario.useGrant && grant
      ? { authorization: { grant_id: grant.id, attempt_id: attemptId } }
      : {}),
    action: {
      kind: 'refund', operation: 'issue_refund', principal_id: PRINCIPAL_ID,
      amount: { amount_minor: scenario.amountMinor, currency: 'USD' },
      counterparty: { id: CUSTOMER_ID, display_name: 'Example Customer', kind: 'customer', country: 'US', metadata: {} },
      rail: 'payment_http', memo: 'Refund order_123: damaged_item',
      metadata: { order_id: 'order_123', reason: 'damaged_item' },
    },
    evidence: [{
      source: 'customer_backend', source_id: 'refund_eligibility_check_789', kind: 'refund_eligibility',
      observed_at: '2026-07-06T10:00:00.000Z',
      metadata: { order_exists: true, payment_captured: true, refund_window_open: true },
    }],
  };
}

export async function runRefundDemo(deps: {
  client: FinancialDemoClient;
  providerCallCount: () => number;
}): Promise<ScenarioResult[]> {
  const grant = await createRefundGrant(deps.client);
  const results: ScenarioResult[] = [];
  for (const scenario of REFUND_SCENARIOS) {
    const before = deps.providerCallCount();
    const request = buildRefundRequest(scenario, grant);
    const first = await deps.client.guardPayment(request);
    const replay = scenario.duplicateSubmit ? await deps.client.guardPayment(request) : first;
    let current = replay;
    if (current.authorization_effect === 'permit' && request.authorization) {
      const attemptId = `${request.authorization.attempt_id}:execute`;
      current = await deps.client.executeAction(current.id, {
        authorization: { grant_id: grant.id, attempt_id: attemptId },
        attempt_id: attemptId,
      });
    }
    const authorizationReceipt = current.authorization_receipt_id
      ? await deps.client.getAuthorizationReceipt(current.authorization_receipt_id)
      : undefined;
    let executionReceiptExported = false;
    if (current.execution_status === 'succeeded') {
      const receipt = await deps.client.getReceipt(current.id);
      executionReceiptExported = receipt.action_id === current.id;
      await deps.client.recordActionOutcome(current.id, {
        action_id: current.id, status: 'succeeded', reversal_capability: 'provider_reversal',
        recovery_status: 'not_needed', provider_status: 'succeeded', provider_reference: `provider_${current.id}`,
        occurred_at: current.updated_at, metadata: { demo: true },
      });
    }
    const outcomes = await deps.client.listActionOutcomes(current.id);
    results.push({
      key: scenario.key, label: scenario.label,
      authorizationEffect: current.authorization_effect,
      executionStatus: current.execution_status,
      actionId: current.id,
      providerCalls: deps.providerCallCount() - before,
      authorizationReceiptExported: authorizationReceipt?.domain === 'financial',
      executionReceiptExported,
      outcomeRecorded: outcomes.outcomes.length > 0,
      duplicateReusedAction: replay.id === first.id,
    });
  }
  return results;
}
