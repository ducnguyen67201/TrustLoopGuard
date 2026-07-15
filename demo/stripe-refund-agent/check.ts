import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import type {
  AuthorizationGrant, AuthorizationGrantListResponse, CreateAuthorizationGrantRequest,
  CreateFinancialActionRequest, FinancialOperation, FinancialOperationSpec,
  FinancialActionRecord, FinancialReceipt,
} from '@trustloopguard/sdk';

import { executeRefundTool, prepareRefundTool, type RefundAgentClient } from './core';
import { resetOrderDatabase } from './order-db';
import { searchOrder } from './orders';
import { handleProviderPayment, providerApiKey } from './provider';
import { requireStripeTestKey } from './stripe';
import { DEMO_ORDER_ID, type StripeRefundProviderRequest, type StripeRefundProviderResponse } from './types';

process.env.STRIPE_REFUND_AGENT_DB = join(mkdtempSync(join(tmpdir(), 'tlg-stripe-refund-agent-')), 'orders.sqlite');

async function main(): Promise<void> {
  resetOrderDatabase();
  const search = searchOrder({ orderId: DEMO_ORDER_ID });
  assert.equal(search.found, true);

  const client = new MockRefundClient();
  const prepared = await prepareRefundTool(
    { orderId: DEMO_ORDER_ID, amountMinor: 7_500, reason: 'damaged item' },
    client,
  );
  assert.equal(prepared.status, 'permit');
  assert.equal(prepared.request.authorization?.grant_id, 'grant_refund_demo');
  assert.equal(prepared.request.action.amount.amount_minor, 7_500n);

  const executed = await executeRefundTool(prepared.action.id, client);
  assert.equal(executed.status, 'succeeded');
  assert.equal(client.executions, 1);
  assert.ok(executed.receipt);

  const rejected = await handleProviderPayment('Bearer wrong', providerRequest());
  assert.equal(rejected.statusCode, 401);
  const accepted = await handleProviderPayment(`Bearer ${providerApiKey()}`, providerRequest());
  assert.equal(accepted.statusCode, 200);
  assertProviderSuccess(accepted.body);
  assert.equal(accepted.body.status, 'succeeded');
  assert.throws(() => requireStripeTestKey('sk_live_never'), /test-mode key/);

  process.stdout.write('stripe refund agent check: all assertions passed\n');
}

class MockRefundClient implements RefundAgentClient {
  executions = 0;
  private grant?: AuthorizationGrant;
  private sequence = 1;
  private readonly actions = new Map<string, FinancialActionRecord>();
  private readonly receipts = new Map<string, FinancialReceipt>();

  async createGrant(req: CreateAuthorizationGrantRequest): Promise<AuthorizationGrant> {
    const now = timestamp();
    this.grant = {
      id: 'grant_refund_demo', workspace_id: 'demo_workspace', environment_id: 'production',
      principal_id: req.principal_id, domain: req.domain, capability: req.capability,
      mode: 'scoped', status: 'active', source: 'user_intent', scope: req.scope,
      fingerprint_version: 1, requirement_ids: req.requirement_ids, use_count: 0,
      created_by: 'demo-admin', created_at: now, updated_at: now,
    };
    return this.grant;
  }

  async listGrants(): Promise<AuthorizationGrantListResponse> {
    return { grants: this.grant ? [this.grant] : [] };
  }

  financialOperation<Input, Facts>(spec: FinancialOperationSpec<Input, Facts>): FinancialOperation<Input, Facts> {
    const buildRequest = (input: Input, facts?: Facts): CreateFinancialActionRequest => {
      const resolvedFacts = facts as Facts;
      const action: CreateFinancialActionRequest['action'] = {
        kind: spec.kind, operation: spec.operation, principal_id: spec.principalId,
        amount: spec.amount(input, resolvedFacts), rail: spec.rail,
        metadata: spec.metadata?.(input, resolvedFacts) ?? {},
      };
      const counterparty = spec.counterparty?.(input, resolvedFacts);
      if (counterparty) action.counterparty = counterparty;
      const memo = spec.memo?.(input, resolvedFacts);
      if (memo) action.memo = memo;
      const authorization = spec.authorization?.(input, resolvedFacts);
      return {
        idempotency_key: spec.idempotencyKey(input, resolvedFacts), execute: false,
        ...(authorization ? { authorization } : {}), action,
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
    const permitted = req.authorization?.grant_id === this.grant?.id && req.action.amount.amount_minor <= 10_000n;
    const now = timestamp();
    const record: FinancialActionRecord = {
      id, workspace_id: 'demo_workspace', environment_id: 'production',
      authorization_intent_id: `intent_${id}`, authorization_receipt_id: `authorization_${id}`,
      authorization_effect: permitted ? 'permit' : 'deny',
      authorization_status: permitted ? 'authorized' : 'denied', execution_status: 'not_started',
      action: { ...req.action, id }, evidence: req.evidence, created_at: now, updated_at: now,
    };
    this.actions.set(id, record);
    return record;
  }

  async getFinancialAction(id: string): Promise<FinancialActionRecord> { return this.requireAction(id); }

  async executeAction(
    id: string,
    request: { authorization: { grant_id: string; attempt_id: string }; attempt_id: string },
  ): Promise<FinancialActionRecord> {
    const current = this.requireAction(id);
    if (request.authorization.grant_id !== this.grant?.id) return current;
    this.executions += 1;
    const updated = { ...current, execution_status: 'succeeded' as const, updated_at: timestamp() };
    this.actions.set(id, updated);
    this.receipts.set(id, {
      id, action_id: id, authorization_receipt_id: current.authorization_receipt_id!,
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

function providerRequest(): StripeRefundProviderRequest {
  return {
    action_id: 'financial_action_123', kind: 'refund', amount_minor: 7_500, currency: 'USD',
    metadata: { payment_intent_id: 'pi_demo_seeded_refund', order_id: DEMO_ORDER_ID, reason: 'damaged_item' },
  };
}

function assertProviderSuccess(
  body: StripeRefundProviderResponse | { error: string },
): asserts body is StripeRefundProviderResponse {
  assert.equal('status' in body, true);
}

function timestamp(): string { return '2026-07-06T10:00:00.000Z'; }

main().catch((error) => {
  process.stderr.write(`stripe refund agent check failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
