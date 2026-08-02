import assert from 'node:assert/strict';
import test from 'node:test';

import type {
  AuthorizationGrant,
  AuthorizationGrantListResponse,
  CreateAuthorizationGrantRequest,
  CreateFinancialActionRequest,
  FinancialActionRecord,
  FinancialOperation,
  FinancialOperationSpec,
  FinancialReceipt,
} from '@featherlane-ai/sdk';

import { executeRefundTool, prepareRefundTool, type RefundAgentClient } from './core';
import { resetOrderDatabase } from './order-db';
import { DEMO_ORDER_ID } from './types';

test('public refund runtime lets policy hold approval-threshold refunds', async () => {
  resetOrderDatabase();
  const client = new RuntimeOnlyRefundClient('grant_preprovisioned_refund_demo');

  const prepared = await prepareRefundTool(
    { orderId: DEMO_ORDER_ID, amountMinor: 7_500, reason: 'damaged item' },
    client,
    undefined,
    { grantId: 'grant_preprovisioned_refund_demo' },
  );

  assert.equal(prepared.status, 'require_approval');
  assert.equal(prepared.request.authorization, undefined);
  assert.equal(client.adminGrantCalls, 0);

  const executed = await executeRefundTool(prepared.action.id, client, undefined, {
    grantId: 'grant_preprovisioned_refund_demo',
  });

  assert.equal(executed.status, 'require_approval');
  assert.equal(client.executions, 0);
  assert.equal(client.adminGrantCalls, 0);
});

test('public refund runtime executes policy-permitted refunds without a grant claim', async () => {
  resetOrderDatabase();
  const client = new RuntimeOnlyRefundClient('grant_preprovisioned_refund_demo');

  const prepared = await prepareRefundTool(
    { orderId: DEMO_ORDER_ID, amountMinor: 2_500, reason: 'damaged item' },
    client,
    undefined,
    { grantId: 'grant_preprovisioned_refund_demo' },
  );

  assert.equal(prepared.status, 'permit');
  assert.equal(prepared.request.authorization, undefined);

  const executed = await executeRefundTool(prepared.action.id, client, undefined, {
    grantId: 'grant_preprovisioned_refund_demo',
  });

  assert.equal(executed.status, 'succeeded');
  assert.equal(client.executions, 1);
  assert.deepEqual(client.executionRequests, [
    { attempt_id: `stripe-refund-agent:execute:${prepared.action.id}` },
  ]);
  assert.equal(client.adminGrantCalls, 0);
});

class RuntimeOnlyRefundClient implements RefundAgentClient {
  adminGrantCalls = 0;
  executions = 0;
  executionRequests: Array<{
    authorization?: { grant_id: string; attempt_id: string };
    attempt_id?: string;
  }> = [];
  private sequence = 1;
  private readonly actions = new Map<string, FinancialActionRecord>();
  private readonly receipts = new Map<string, FinancialReceipt>();

  constructor(private readonly configuredGrantId: string) {}

  async createGrant(_req: CreateAuthorizationGrantRequest): Promise<AuthorizationGrant> {
    this.adminGrantCalls += 1;
    throw new Error('public runtime must not create authorization grants');
  }

  async listGrants(): Promise<AuthorizationGrantListResponse> {
    this.adminGrantCalls += 1;
    throw new Error('public runtime must not list authorization grants');
  }

  financialOperation<Input, Facts>(
    spec: FinancialOperationSpec<Input, Facts>,
  ): FinancialOperation<Input, Facts> {
    const buildRequest = (input: Input, facts?: Facts): CreateFinancialActionRequest => {
      const resolvedFacts = facts as Facts;
      const action: CreateFinancialActionRequest['action'] = {
        kind: spec.kind,
        operation: spec.operation,
        principal_id: spec.principalId,
        amount: spec.amount(input, resolvedFacts),
        rail: spec.rail,
        metadata: spec.metadata?.(input, resolvedFacts) ?? {},
      };
      const counterparty = spec.counterparty?.(input, resolvedFacts);
      if (counterparty) action.counterparty = counterparty;
      const memo = spec.memo?.(input, resolvedFacts);
      if (memo) action.memo = memo;
      const authorization = spec.authorization?.(input, resolvedFacts);
      return {
        idempotency_key: spec.idempotencyKey(input, resolvedFacts),
        execute: false,
        ...(authorization ? { authorization } : {}),
        action,
        evidence: spec.evidence?.(input, resolvedFacts) ?? [],
      };
    };
    return {
      buildRequest,
      verify: async (input, facts) => this.guardPayment(buildRequest(input, facts)),
    };
  }

  async guardPayment(req: CreateFinancialActionRequest): Promise<FinancialActionRecord> {
    const id = `refund_action_${this.sequence++}`;
    const amount = req.action.amount.amount_minor;
    const permitted = req.authorization === undefined && amount < 5_000n;
    const held = req.authorization === undefined && amount >= 5_000n && amount <= 10_000n;
    const now = '2026-07-06T10:00:00.000Z';
    const record: FinancialActionRecord = {
      id,
      workspace_id: 'demo_workspace',
      environment_id: 'production',
      authorization_intent_id: `intent_${id}`,
      authorization_receipt_id: `authorization_${id}`,
      authorization_effect: permitted ? 'permit' : held ? 'require_approval' : 'deny',
      authorization_status: permitted ? 'authorized' : held ? 'pending_approval' : 'denied',
      execution_status: 'not_started',
      state: permitted ? 'authorized' : held ? 'held_for_approval' : 'blocked',
      action: { ...req.action, id },
      evidence: req.evidence,
      created_at: now,
      updated_at: now,
    };
    this.actions.set(id, record);
    return record;
  }

  async getFinancialAction(id: string): Promise<FinancialActionRecord> {
    return this.requireAction(id);
  }

  async executeAction(
    id: string,
    request: { authorization?: { grant_id: string; attempt_id: string }; attempt_id?: string } = {},
  ): Promise<FinancialActionRecord> {
    const current = this.requireAction(id);
    this.executionRequests.push(request);
    if (current.authorization_effect !== 'permit') return current;
    if (request.authorization !== undefined && request.authorization.grant_id !== this.configuredGrantId) {
      return current;
    }
    this.executions += 1;
    const updated = {
      ...current,
      execution_status: 'succeeded' as const,
      state: 'executed' as const,
      updated_at: '2026-07-06T10:00:01.000Z',
    };
    this.actions.set(id, updated);
    this.receipts.set(id, {
      id,
      action_id: id,
      authorization_receipt_id: current.authorization_receipt_id!,
      ledger_event_ids: [`${id}:executed`],
      proof: { provider: { reference: `simulated_re_${id}`, status: 'succeeded' } },
      created_at: updated.updated_at,
    });
    return updated;
  }

  async getReceipt(id: string): Promise<FinancialReceipt> {
    const receipt = this.receipts.get(id);
    if (!receipt) throw new Error(`missing receipt ${id}`);
    return receipt;
  }

  private requireAction(id: string): FinancialActionRecord {
    const action = this.actions.get(id);
    if (!action) throw new Error(`missing action ${id}`);
    return action;
  }
}
