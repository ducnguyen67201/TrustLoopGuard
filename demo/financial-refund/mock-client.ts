import type {
  AuthorizationGrant, AuthorizationReceipt, CreateAuthorizationGrantRequest,
  CreateFinancialActionRequest, FinancialActionOutcome, FinancialActionRecord,
  FinancialOutcomeListResponse, FinancialReceipt,
} from '@trustloopguard/sdk';

import type { FinancialDemoClient } from './core';

export class MockFinancialRefundClient implements FinancialDemoClient {
  private sequence = 1;
  private calls = 0;
  private grant?: AuthorizationGrant;
  private readonly actions = new Map<string, FinancialActionRecord>();
  private readonly idempotency = new Map<string, string>();
  private readonly authorizationReceipts = new Map<string, AuthorizationReceipt>();
  private readonly executionReceipts = new Map<string, FinancialReceipt>();
  private readonly outcomes = new Map<string, FinancialActionOutcome[]>();

  providerCallCount(): number { return this.calls; }

  async createGrant(req: CreateAuthorizationGrantRequest): Promise<AuthorizationGrant> {
    const now = timestamp();
    this.grant = {
      id: 'grant_refund_demo', workspace_id: 'demo_workspace', environment_id: 'production',
      principal_id: req.principal_id, domain: req.domain, capability: req.capability,
      mode: 'scoped', status: 'active', source: 'user_intent', scope: req.scope,
      fingerprint_version: 1, requirement_ids: req.requirement_ids, max_uses: req.max_uses,
      use_count: 0, created_by: 'demo-admin', created_at: now, updated_at: now,
    };
    return this.grant;
  }

  async guardPayment(req: CreateFinancialActionRequest): Promise<FinancialActionRecord> {
    const existing = this.idempotency.get(req.idempotency_key);
    if (existing) return this.requireAction(existing);
    const id = `financial_demo_${this.sequence++}`;
    const permitted = req.authorization?.grant_id === this.grant?.id && req.action.amount.amount_minor <= 10_000n;
    const receiptId = `authorization_${id}`;
    const now = timestamp();
    const record: FinancialActionRecord = {
      id, workspace_id: 'demo_workspace', environment_id: 'production',
      authorization_intent_id: `intent_${id}`, authorization_receipt_id: receiptId,
      authorization_effect: permitted ? 'permit' : 'require_approval',
      authorization_status: permitted ? 'authorized' : 'pending_approval',
      execution_status: 'not_started', action: { ...req.action, id }, evidence: req.evidence,
      created_at: now, updated_at: now,
    };
    this.actions.set(id, record);
    this.idempotency.set(req.idempotency_key, id);
    this.authorizationReceipts.set(receiptId, {
      id: receiptId, intent_id: record.authorization_intent_id, domain: 'financial',
      effect: record.authorization_effect, intent_status: record.authorization_status,
      subject_hash: `sha256:${id}`, reason: permitted ? 'saved grant and current policy permit' : 'grant required',
      findings: [], policy_versions: ['refund-controls'], grant_id: permitted ? this.grant?.id : undefined,
      domain_evidence: { domain: 'financial', evidence: { action_id: id } }, created_at: now,
    });
    return record;
  }

  async executeAction(
    actionId: string,
    request: { authorization: { grant_id: string; attempt_id: string }; attempt_id: string },
  ): Promise<FinancialActionRecord> {
    const current = this.requireAction(actionId);
    if (current.execution_status === 'succeeded') return current;
    if (current.authorization_effect !== 'permit' || request.authorization.grant_id !== this.grant?.id) return current;
    this.calls += 1;
    const updated = { ...current, execution_status: 'succeeded' as const, updated_at: timestamp() };
    this.actions.set(actionId, updated);
    this.executionReceipts.set(actionId, {
      id: actionId, action_id: actionId, authorization_receipt_id: current.authorization_receipt_id!,
      ledger_event_ids: [`${actionId}:reserved`, `${actionId}:executed`],
      proof: { provider_reference: `provider_${actionId}` }, created_at: updated.updated_at,
    });
    return updated;
  }

  async getAuthorizationReceipt(id: string): Promise<AuthorizationReceipt> {
    const receipt = this.authorizationReceipts.get(id);
    if (!receipt) throw new Error(`authorization receipt not found: ${id}`);
    return receipt;
  }
  async getReceipt(id: string): Promise<FinancialReceipt> {
    const receipt = this.executionReceipts.get(id);
    if (!receipt) throw new Error(`execution receipt not found: ${id}`);
    return receipt;
  }
  async recordActionOutcome(id: string, outcome: FinancialActionOutcome): Promise<FinancialActionOutcome> {
    this.outcomes.set(id, [outcome, ...(this.outcomes.get(id) ?? [])]);
    return outcome;
  }
  async listActionOutcomes(id: string): Promise<FinancialOutcomeListResponse> {
    return { outcomes: this.outcomes.get(id) ?? [] };
  }
  private requireAction(id: string): FinancialActionRecord {
    const action = this.actions.get(id);
    if (!action) throw new Error(`action not found: ${id}`);
    return action;
  }
}

function timestamp(): string { return '2026-07-06T10:00:00.000Z'; }
