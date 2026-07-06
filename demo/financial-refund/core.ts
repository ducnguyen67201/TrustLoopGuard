import type {
  CreateFinancialActionRequest,
  CreateFinancialMandateRequest,
  FinancialActionOutcome,
  FinancialActionRecord,
  FinancialActionStatus,
  FinancialMandate,
  FinancialOutcomeListResponse,
  FinancialReceipt,
} from '@trustloopguard/sdk';

export interface FinancialDemoClient {
  createMandate(req: CreateFinancialMandateRequest): Promise<FinancialMandate>;
  guardPayment(req: CreateFinancialActionRequest): Promise<FinancialActionRecord>;
  approveAction(actionId: string): Promise<FinancialActionRecord>;
  denyAction(actionId: string): Promise<FinancialActionRecord>;
  executeAction(actionId: string): Promise<FinancialActionRecord>;
  getReceipt(receiptId: string): Promise<FinancialReceipt>;
  recordActionOutcome(
    actionId: string,
    outcome: FinancialActionOutcome,
  ): Promise<FinancialActionOutcome>;
  listActionOutcomes(actionId: string): Promise<FinancialOutcomeListResponse>;
}

export type ScenarioKey =
  | 'normal_allow'
  | 'hold_then_approve'
  | 'hold_then_deny'
  | 'duplicate_idempotency'
  | 'missing_mandate';

export interface RefundScenario {
  key: ScenarioKey;
  label: string;
  amountMinor: bigint;
  useMandate: boolean;
  duplicateSubmit?: boolean;
  approval?: 'approve' | 'deny';
}

export interface ScenarioResult {
  key: ScenarioKey;
  label: string;
  initialStatus: FinancialActionStatus;
  finalStatus: FinancialActionStatus;
  actionId: string;
  providerCalls: number;
  receiptExported: boolean;
  outcomeRecorded: boolean;
  duplicateReusedAction: boolean;
}

type ScenarioRun = Omit<ScenarioResult, 'providerCalls'>;

const PRINCIPAL_ID = 'refund-bot';
const CUSTOMER_ID = 'cust_456';
const ORDER_ID = 'order_123';
const PAYMENT_METHOD_ID = 'card_original_abc';

export const REFUND_SCENARIOS: RefundScenario[] = [
  {
    key: 'normal_allow',
    label: 'refund $40 under approval threshold',
    amountMinor: 4_000n,
    useMandate: true,
  },
  {
    key: 'hold_then_approve',
    label: 'refund $75 held, approved, then executed',
    amountMinor: 7_500n,
    useMandate: true,
    approval: 'approve',
  },
  {
    key: 'hold_then_deny',
    label: 'refund $80 held, denied, no provider call',
    amountMinor: 8_000n,
    useMandate: true,
    approval: 'deny',
  },
  {
    key: 'duplicate_idempotency',
    label: 'duplicate retry reuses action id',
    amountMinor: 3_500n,
    useMandate: true,
    duplicateSubmit: true,
  },
  {
    key: 'missing_mandate',
    label: 'refund without mandate is denied',
    amountMinor: 3_000n,
    useMandate: false,
  },
];

export async function createRefundMandate(client: FinancialDemoClient): Promise<FinancialMandate> {
  return client.createMandate({
    id: 'mandate_refund_support_v1',
    version: 1,
    principal_id: PRINCIPAL_ID,
    scope: {
      action_kinds: ['refund'],
      rails: ['payment_http'],
      max_amount_minor: 10_000,
      currency: 'USD',
      counterparty_ids: [CUSTOMER_ID],
    },
    metadata: {
      source: 'demo',
      workflow: 'support_refund',
    },
  });
}

export function buildRefundRequest(
  scenario: RefundScenario,
  mandate: FinancialMandate | null,
): CreateFinancialActionRequest {
  return {
    idempotency_key: `financial-refund:${scenario.key}`,
    execute: scenario.approval === undefined,
    action: {
      kind: 'refund',
      principal_id: PRINCIPAL_ID,
      amount: {
        amount_minor: scenario.amountMinor,
        currency: 'USD',
      },
      counterparty: {
        id: CUSTOMER_ID,
        display_name: 'Example Customer',
        kind: 'customer',
        country: 'US',
        metadata: {
          original_payment_method_id: PAYMENT_METHOD_ID,
        },
      },
      rail: 'payment_http',
      mandate:
        scenario.useMandate && mandate ? { id: mandate.id, version: mandate.version } : undefined,
      memo: `Refund ${ORDER_ID}: damaged_item`,
      metadata: {
        order_id: ORDER_ID,
        customer_id: CUSTOMER_ID,
        reason: 'damaged_item',
        destination_payment_method_id: PAYMENT_METHOD_ID,
      },
    },
    evidence: [
      {
        source: 'customer_backend',
        source_id: 'refund_eligibility_check_789',
        kind: 'refund_eligibility',
        observed_at: '2026-07-06T10:00:00.000Z',
        metadata: {
          order_exists: true,
          payment_captured: true,
          refundable_balance_minor: 10_000,
          refund_window_open: true,
          destination_is_original_payment_method: true,
          no_duplicate_refund: true,
        },
      },
    ],
  };
}

export async function runRefundScenario(
  client: FinancialDemoClient,
  scenario: RefundScenario,
  mandate: FinancialMandate | null,
): Promise<ScenarioRun> {
  const request = buildRefundRequest(scenario, mandate);
  const first = await client.guardPayment(request);
  const replay = scenario.duplicateSubmit ? await client.guardPayment(request) : first;
  let current = replay;

  if (current.status === 'held' && scenario.approval === 'approve') {
    current = await client.approveAction(current.id);
    current = await client.executeAction(current.id);
  } else if (current.status === 'held' && scenario.approval === 'deny') {
    current = await client.denyAction(current.id);
  }

  let receiptExported = false;
  if (current.status === 'executed') {
    await client.recordActionOutcome(current.id, {
      action_id: current.id,
      status: 'succeeded',
      reversal_capability: 'provider_reversal',
      recovery_status: 'not_needed',
      provider_status: 'succeeded',
      provider_reference: `provider_${current.id}`,
      occurred_at: current.updated_at,
      metadata: {
        demo: true,
        reversible_by: 'provider_refund_reversal',
      },
    });
    const receipt = await client.getReceipt(current.id);
    receiptExported = receipt.action_id === current.id && receipt.ledger_event_ids.length > 0;
  }

  const outcomes = await client.listActionOutcomes(current.id);
  return {
    key: scenario.key,
    label: scenario.label,
    initialStatus: first.status,
    finalStatus: current.status,
    actionId: current.id,
    receiptExported,
    outcomeRecorded: outcomes.outcomes.length > 0,
    duplicateReusedAction: replay.id === first.id,
  };
}

export async function runRefundDemo(deps: {
  client: FinancialDemoClient;
  providerCallCount: () => number;
}): Promise<ScenarioResult[]> {
  const mandate = await createRefundMandate(deps.client);
  const results: ScenarioResult[] = [];
  for (const scenario of REFUND_SCENARIOS) {
    const before = deps.providerCallCount();
    const result = await runRefundScenario(deps.client, scenario, mandate);
    results.push({ ...result, providerCalls: deps.providerCallCount() - before });
  }
  return results;
}
