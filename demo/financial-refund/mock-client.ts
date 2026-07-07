import type {
  CreateFinancialActionRequest,
  CreateFinancialMandateRequest,
  FinancialActionDecisionReceipt,
  FinancialActionOutcome,
  FinancialActionRecord,
  FinancialActionPrecondition,
  FinancialActionStatus,
  FinancialMandate,
  FinancialOutcomeListResponse,
  FinancialReceipt,
} from '@trustloopguard/sdk';

import type { FinancialDemoClient } from './core';

const WORKSPACE_ID = 'demo_financial_workspace';
const HOLD_THRESHOLD_MINOR = 5_000n;
const PER_ACTION_CAP_MINOR = 10_000n;

export class MockFinancialRefundClient implements FinancialDemoClient {
  private sequence = 1;
  private readonly mandates = new Map<string, FinancialMandate>();
  private readonly actions = new Map<string, FinancialActionRecord>();
  private readonly idempotency = new Map<string, string>();
  private readonly receipts = new Map<string, FinancialReceipt>();
  private readonly outcomes = new Map<string, FinancialActionOutcome[]>();
  private calls = 0;

  providerCallCount(): number {
    return this.calls;
  }

  async createMandate(req: CreateFinancialMandateRequest): Promise<FinancialMandate> {
    const now = timestamp();
    const mandate: FinancialMandate = {
      id: req.id ?? `mandate_${this.sequence++}`,
      workspace_id: WORKSPACE_ID,
      version: req.version ?? 1,
      status: 'active',
      principal_id: req.principal_id,
      scope: req.scope,
      metadata: req.metadata,
      starts_at: req.starts_at,
      expires_at: req.expires_at,
      created_at: now,
      updated_at: now,
    };
    this.mandates.set(mandate.id, mandate);
    return mandate;
  }

  async guardPayment(req: CreateFinancialActionRequest): Promise<FinancialActionRecord> {
    const existingId = this.idempotency.get(req.idempotency_key);
    if (existingId) return this.requireAction(existingId);

    const id = `financial_demo_${String(this.sequence++).padStart(3, '0')}`;
    const status = this.initialStatus(req);
    const now = timestamp();
    const record: FinancialActionRecord = {
      id,
      workspace_id: WORKSPACE_ID,
      status,
      action: { ...req.action, id },
      evidence: req.evidence,
      created_at: now,
      updated_at: now,
    };
    this.actions.set(id, record);
    this.idempotency.set(req.idempotency_key, id);

    if (req.execute && status === 'authorized') {
      return this.executeAction(id);
    }
    return record;
  }

  async approveAction(actionId: string): Promise<FinancialActionRecord> {
    const action = this.requireAction(actionId);
    if (action.status !== 'held') return action;
    return this.updateStatus(actionId, 'authorized');
  }

  async denyAction(actionId: string): Promise<FinancialActionRecord> {
    const action = this.requireAction(actionId);
    if (action.status === 'executed') return action;
    return this.updateStatus(actionId, 'denied');
  }

  async executeAction(actionId: string): Promise<FinancialActionRecord> {
    const action = this.requireAction(actionId);
    if (action.status === 'executed') return action;
    if (action.status !== 'authorized') return action;

    this.calls += 1;
    const executed = this.updateStatus(actionId, 'executed');
    this.receipts.set(actionId, {
      id: actionId,
      action_id: actionId,
      trace_id: `trace_${actionId}`,
      ledger_event_ids: [`${actionId}:reserved`, `${actionId}:executed`],
      proof: {
        provider: 'mock_payment_http',
        provider_reference: `provider_${actionId}`,
        policy_snapshot: {
          per_action_cap_minor: Number(PER_ACTION_CAP_MINOR),
          hold_threshold_minor: Number(HOLD_THRESHOLD_MINOR),
        },
        mandate_ref: executed.action.mandate ?? null,
        evidence_refs: executed.evidence.map((evidence) => evidence.source_id),
      },
      created_at: executed.updated_at,
    });
    return executed;
  }

  async getReceipt(receiptId: string): Promise<FinancialReceipt> {
    const receipt = this.receipts.get(receiptId);
    if (!receipt) throw new Error(`receipt not found: ${receiptId}`);
    return receipt;
  }

  async getFinancialDecisionReceipt(actionId: string): Promise<FinancialActionDecisionReceipt> {
    const action = this.requireAction(actionId);
    const hasScope = Boolean(action.action.mandate && this.hasValidMandateAction(action));
    const amountMinor = action.action.amount.amount_minor;
    const risks =
      action.status === 'held'
        ? [
            {
              code: 'amount_above_auto_approve_threshold' as const,
              severity: 'high' as const,
              reason: 'amount at or above hold threshold',
              policy_id: 'refund-controls',
              source: 'financial_policy',
            },
          ]
        : !hasScope
          ? [
              {
                code: 'missing_authorization_scope' as const,
                severity: 'high' as const,
                reason: 'authorization scope required before execution',
                source: 'authorization_scope',
              },
            ]
          : amountMinor > PER_ACTION_CAP_MINOR
            ? [
                {
                  code: 'amount_over_per_transaction_cap' as const,
                  severity: 'high' as const,
                  reason: 'amount over per-transaction cap',
                  policy_id: 'refund-controls',
                  source: 'financial_policy',
                },
              ]
            : [];
    const receipt = this.receipts.get(actionId);
    return {
      schema: 'financial_action_decision_receipt.v1',
      action_id: action.id,
      decision:
        action.status === 'held'
          ? 'hold'
          : action.status === 'denied' || action.status === 'failed'
            ? 'block'
            : 'allow',
      status: action.status,
      reason:
        action.status === 'held'
          ? 'valid refund, but above threshold so human approval required'
          : risks[0]?.reason ?? 'financial action passed authorization checks',
      amount: action.action.amount,
      operation: action.action.operation,
      principal_id: action.action.principal_id,
      counterparty: action.action.counterparty,
      authorization_scope: {
        checked: Boolean(action.action.mandate),
        result: hasScope ? 'passed' : 'missing',
        scope_ref: action.action.mandate,
        source: 'financial_authorization_service',
        reason: hasScope
          ? 'support agent may refund up to USD 100.00'
          : 'authorization scope required before execution',
      },
      evidence: REQUIRED_PRECONDITIONS.map((precondition) => ({
        precondition,
        status: evidencePassed(action, precondition) ? 'passed' : 'missing',
        evidence_source_id: action.evidence[0]?.source_id,
      })),
      risks,
      execution: {
        status: receipt ? 'executed' : 'not_started',
        receipt_id: receipt?.id,
        ledger_event_ids: receipt?.ledger_event_ids ?? [],
      },
      created_at: action.created_at,
      updated_at: action.updated_at,
    };
  }

  async recordActionOutcome(
    actionId: string,
    outcome: FinancialActionOutcome,
  ): Promise<FinancialActionOutcome> {
    this.requireAction(actionId);
    const current = this.outcomes.get(actionId) ?? [];
    current.unshift(outcome);
    this.outcomes.set(actionId, current);
    return outcome;
  }

  async listActionOutcomes(actionId: string): Promise<FinancialOutcomeListResponse> {
    this.requireAction(actionId);
    return { outcomes: this.outcomes.get(actionId) ?? [] };
  }

  private initialStatus(req: CreateFinancialActionRequest): FinancialActionStatus {
    if (!this.hasValidMandate(req)) return 'denied';
    if (req.action.kind !== 'refund') return 'denied';
    if (req.action.amount.currency !== 'USD') return 'denied';
    if (req.action.amount.amount_minor <= 0n) return 'denied';
    if (req.action.amount.amount_minor > PER_ACTION_CAP_MINOR) return 'denied';
    if (req.action.amount.amount_minor >= HOLD_THRESHOLD_MINOR) return 'held';
    return 'authorized';
  }

  private hasValidMandate(req: CreateFinancialActionRequest): boolean {
    const mandateRef = req.action.mandate;
    if (!mandateRef) return false;
    const mandate = this.mandates.get(mandateRef.id);
    if (!mandate || mandate.status !== 'active') return false;
    if (mandate.principal_id !== req.action.principal_id) return false;
    return mandateRef.version === undefined || mandateRef.version === mandate.version;
  }

  private hasValidMandateAction(action: FinancialActionRecord): boolean {
    const mandateRef = action.action.mandate;
    if (!mandateRef) return false;
    const mandate = this.mandates.get(mandateRef.id);
    if (!mandate || mandate.status !== 'active') return false;
    if (mandate.principal_id !== action.action.principal_id) return false;
    return mandateRef.version === undefined || mandateRef.version === mandate.version;
  }

  private requireAction(actionId: string): FinancialActionRecord {
    const action = this.actions.get(actionId);
    if (!action) throw new Error(`financial action not found: ${actionId}`);
    return action;
  }

  private updateStatus(
    actionId: string,
    status: FinancialActionStatus,
  ): FinancialActionRecord {
    const action = this.requireAction(actionId);
    const updated: FinancialActionRecord = {
      ...action,
      status,
      updated_at: timestamp(),
    };
    this.actions.set(actionId, updated);
    return updated;
  }
}

const REQUIRED_PRECONDITIONS: FinancialActionPrecondition[] = [
  'order_exists',
  'payment_captured',
  'refund_window_open',
  'destination_is_original_payment_method',
];

function evidencePassed(
  action: FinancialActionRecord,
  precondition: FinancialActionPrecondition,
): boolean {
  return action.evidence.some((evidence) => evidence.metadata?.[precondition] === true);
}

function timestamp(): string {
  return '2026-07-06T10:00:00.000Z';
}
