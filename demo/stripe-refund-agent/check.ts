import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import type {
  CreateFinancialActionRequest,
  FinancialActionRecord,
  FinancialActionStatus,
  FinancialMandate,
  FinancialMandateListResponse,
  FinancialReceipt,
} from '@trustloopguard/sdk';

import { runRefundAgent } from './agent';
import {
  buildRefundActionRequest,
  executeRefundTool,
  prepareRefundTool,
  type RefundAgentClient,
} from './core';
import { resetOrderDatabase } from './order-db';
import { searchOrder } from './orders';
import { handleProviderPayment, providerApiKey } from './provider';
import { requireStripeTestKey } from './stripe';
import {
  DEMO_ORDER_ID,
  REFUND_AGENT_ID,
  type StripeRefundProviderRequest,
  type StripeRefundProviderResponse,
} from './types';

process.env.STRIPE_REFUND_AGENT_DB = join(
  mkdtempSync(join(tmpdir(), 'tlg-stripe-refund-agent-')),
  'orders.sqlite',
);

async function testOrderSearch(): Promise<void> {
  resetOrderDatabase();
  const result = searchOrder({ orderId: DEMO_ORDER_ID });
  assert.equal(result.found, true);
  assert.equal(result.order?.captured, true);
  assert.equal(result.evidence.orderExists, true);
  assert.equal(result.evidence.paymentCaptured, true);
  assert.equal(result.evidence.noDuplicateRefund, true);
}

async function testPrepareRefundBuildsTypedAction(): Promise<void> {
  resetOrderDatabase();
  const client = new MockRefundClient(() => 'authorized');
  const result = await prepareRefundTool(
    { orderId: DEMO_ORDER_ID, amountMinor: 7_500, reason: 'damaged item' },
    client,
  );

  assert.equal(result.request.action.kind, 'refund');
  assert.equal(result.request.action.rail, 'payment_http');
  assert.equal(result.request.action.principal_id, REFUND_AGENT_ID);
  assert.equal(result.request.action.amount.amount_minor, 7_500n);
  assert.equal(result.request.action.metadata?.payment_intent_id, 'pi_demo_seeded_refund');
  assert.equal(result.request.evidence[0]?.kind, 'refund_eligibility');
  assert.equal(client.createdMandates, 1);

  const rebuilt = buildRefundActionRequest(
    { orderId: DEMO_ORDER_ID, amountMinor: 7_500, reason: 'damaged item' },
    searchOrder({ orderId: DEMO_ORDER_ID }),
  );
  assert.equal(rebuilt.idempotency_key, result.request.idempotency_key);
}

async function testOfflineAgentExecutesAuthorizedRefund(): Promise<void> {
  resetOrderDatabase();
  const client = new MockRefundClient(() => 'authorized');
  const result = await runRefundAgent(
    `Refund order ${DEMO_ORDER_ID} for $75 because damaged item.`,
    client,
    { useOpenAI: false },
  );

  assert.deepEqual(
    result.traces.map((trace) => trace.tool),
    ['search_order', 'prepare_refund', 'execute_refund'],
  );
  assert.equal(client.executions, 1);
  assert.match(result.finalMessage, /Receipt/);
  assert.ok(result.receiptId);

  const after = searchOrder({ orderId: DEMO_ORDER_ID });
  assert.equal(after.order?.refundableBalanceMinor, 2_500);
  assert.equal(after.evidence.noDuplicateRefund, false);
}

async function testHeldActionDoesNotExecute(): Promise<void> {
  resetOrderDatabase();
  const client = new MockRefundClient(() => 'held');
  const prepared = await prepareRefundTool(
    { orderId: DEMO_ORDER_ID, amountMinor: 7_500, reason: 'damaged item' },
    client,
  );
  const result = await executeRefundTool(prepared.action.id, client);

  assert.equal(result.status, 'held');
  assert.equal(client.executions, 0);
  assert.match(result.message, /held for approval/);

  const after = searchOrder({ orderId: DEMO_ORDER_ID });
  assert.equal(after.order?.refundableBalanceMinor, 10_000);
  assert.equal(after.evidence.noDuplicateRefund, true);
}

async function testProviderAuthAndSimulation(): Promise<void> {
  const request = providerRequest();
  const rejected = await handleProviderPayment('Bearer wrong', request);
  assert.equal(rejected.statusCode, 401);

  const oldKey = process.env.STRIPE_SECRET_KEY;
  try {
    delete process.env.STRIPE_SECRET_KEY;
    const accepted = await handleProviderPayment(`Bearer ${providerApiKey()}`, request);
    assert.equal(accepted.statusCode, 200);
    assertProviderSuccess(accepted.body);
    assert.equal(accepted.body.status, 'succeeded');
    assert.equal(accepted.body.mode, 'simulated');
  } finally {
    restoreEnv('STRIPE_SECRET_KEY', oldKey);
  }
}

async function testStripeSafetyAndMapping(): Promise<void> {
  assert.throws(() => requireStripeTestKey('sk_live_never'), /test-mode key/);

  const oldKey = process.env.STRIPE_SECRET_KEY;
  let postedBody = '';
  try {
    process.env.STRIPE_SECRET_KEY = 'sk_test_demo';
    const accepted = await handleProviderPayment(`Bearer ${providerApiKey()}`, providerRequest(), async (_url, init) => {
      postedBody = String(init.body);
      return new Response(JSON.stringify({ id: 're_test_123', status: 'succeeded' }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    });

    assert.equal(accepted.statusCode, 200);
    assertProviderSuccess(accepted.body);
    assert.equal(accepted.body.provider_reference, 're_test_123');
    assert.equal(accepted.body.mode, 'stripe-test');
    assert.match(postedBody, /payment_intent=pi_demo_seeded_refund/);
    assert.match(postedBody, /amount=7500/);
  } finally {
    restoreEnv('STRIPE_SECRET_KEY', oldKey);
  }
}

function providerRequest(): StripeRefundProviderRequest {
  return {
    action_id: 'financial_action_123',
    kind: 'refund',
    amount_minor: 7_500,
    currency: 'USD',
    metadata: {
      payment_intent_id: 'pi_demo_seeded_refund',
      order_id: DEMO_ORDER_ID,
      reason: 'damaged_item',
    },
  };
}

function restoreEnv(name: string, value: string | undefined): void {
  if (value === undefined) {
    delete process.env[name];
  } else {
    process.env[name] = value;
  }
}

function assertProviderSuccess(
  body: StripeRefundProviderResponse | { error: string },
): asserts body is StripeRefundProviderResponse {
  assert.equal('status' in body, true);
}

class MockRefundClient implements RefundAgentClient {
  createdMandates = 0;
  executions = 0;
  private sequence = 1;
  private readonly mandates = new Map<string, FinancialMandate>();
  private readonly actions = new Map<string, FinancialActionRecord>();
  private readonly receipts = new Map<string, FinancialReceipt>();

  constructor(private readonly statusForRequest: (req: CreateFinancialActionRequest) => FinancialActionStatus) {}

  async createMandate(req: {
    id: string;
    version: number;
    principal_id: string;
    scope: Record<string, string | number | string[]>;
    metadata: Record<string, string>;
  }): Promise<FinancialMandate> {
    this.createdMandates += 1;
    const mandate: FinancialMandate = {
      id: req.id,
      workspace_id: 'demo_workspace',
      version: req.version,
      status: 'active',
      principal_id: req.principal_id,
      scope: req.scope,
      metadata: req.metadata,
      created_at: timestamp(),
      updated_at: timestamp(),
    };
    this.mandates.set(`${mandate.id}:${mandate.version}`, mandate);
    return mandate;
  }

  async listMandates(): Promise<FinancialMandateListResponse> {
    return { mandates: [...this.mandates.values()] };
  }

  async guardPayment(req: CreateFinancialActionRequest): Promise<FinancialActionRecord> {
    const id = `refund_action_${this.sequence++}`;
    const record: FinancialActionRecord = {
      id,
      workspace_id: 'demo_workspace',
      status: this.statusForRequest(req),
      action: { ...req.action, id },
      evidence: req.evidence,
      created_at: timestamp(),
      updated_at: timestamp(),
    };
    this.actions.set(id, record);
    return record;
  }

  async getFinancialAction(actionId: string): Promise<FinancialActionRecord> {
    return this.requireAction(actionId);
  }

  async executeAction(actionId: string): Promise<FinancialActionRecord> {
    const current = this.requireAction(actionId);
    if (current.status !== 'authorized' && current.status !== 'proposed') return current;
    this.executions += 1;
    const executed: FinancialActionRecord = {
      ...current,
      status: 'executed',
      updated_at: timestamp(),
    };
    this.actions.set(actionId, executed);
    this.receipts.set(actionId, {
      id: actionId,
      action_id: actionId,
      trace_id: `trace_${actionId}`,
      ledger_event_ids: [`${actionId}:authorized`, `${actionId}:executed`],
      proof: {
        provider_reference: `simulated_re_${actionId}`,
      },
      created_at: timestamp(),
    });
    return executed;
  }

  async getReceipt(receiptId: string): Promise<FinancialReceipt> {
    const receipt = this.receipts.get(receiptId);
    if (receipt === undefined) throw new Error(`missing receipt ${receiptId}`);
    return receipt;
  }

  private requireAction(actionId: string): FinancialActionRecord {
    const action = this.actions.get(actionId);
    if (action === undefined) throw new Error(`missing action ${actionId}`);
    return action;
  }
}

function timestamp(): string {
  return '2026-07-06T10:00:00.000Z';
}

await testOrderSearch();
await testPrepareRefundBuildsTypedAction();
await testOfflineAgentExecutesAuthorizedRefund();
await testHeldActionDoesNotExecute();
await testProviderAuthAndSimulation();
await testStripeSafetyAndMapping();

process.stdout.write('stripe-refund-agent checks passed\n');
