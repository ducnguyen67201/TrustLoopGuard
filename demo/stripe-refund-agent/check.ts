import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import type {
  CreateFinancialActionRequest,
  FinancialOperation,
  FinancialOperationSpec,
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
import { customerBackendState, resetOrderDatabase } from './order-db';
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
  assert.equal(result.request.action.operation, 'issue_refund');
  assert.equal(result.request.action.metadata?.payment_intent_id, 'pi_demo_seeded_refund');
  assert.equal(result.request.evidence[0]?.kind, 'refund_eligibility');
  assert.equal(client.createdMandates, 1);

  const rebuilt = buildRefundActionRequest(
    { orderId: DEMO_ORDER_ID, amountMinor: 7_500, reason: 'damaged item' },
    searchOrder({ orderId: DEMO_ORDER_ID }),
    client,
  );
  assert.equal(rebuilt.idempotency_key, result.request.idempotency_key);
}

async function testOverRefundStillSubmitsFinancialAction(): Promise<void> {
  resetOrderDatabase();
  let submitted: CreateFinancialActionRequest | undefined;
  const client = new MockRefundClient((req) => {
    submitted = req;
    return 'denied';
  });
  const result = await prepareRefundTool(
    { orderId: DEMO_ORDER_ID, amountMinor: 75_500, reason: 'damaged item' },
    client,
  );

  assert.equal(result.status, 'denied');
  assert.equal(submitted?.action.amount.amount_minor, 75_500n);
  assert.equal(
    submitted?.evidence[0]?.metadata?.amount_lte_refundable_balance,
    false,
  );
  assert.equal(client.createdMandates, 1);
}

async function testOfflineAgentApprovesAndExecutesProposedRefund(): Promise<void> {
  resetOrderDatabase();
  const client = new MockRefundClient(() => 'proposed');
  const result = await runRefundAgent(
    `Refund order ${DEMO_ORDER_ID} for $75 because damaged item.`,
    client,
    { useOpenAI: false },
  );

  assert.deepEqual(
    result.traces.map((trace) => trace.tool),
    ['search_order', 'prepare_refund', 'execute_refund'],
  );
  assert.equal(client.approvals, 1);
  assert.equal(client.executions, 1);
  assert.match(result.finalMessage, /Receipt/);
  assert.ok(result.receiptId);

  const after = searchOrder({ orderId: DEMO_ORDER_ID });
  assert.equal(after.order?.refundableBalanceMinor, 2_500);
  assert.equal(after.evidence.noDuplicateRefund, false);
  const refund = customerRefunds()[0];
  assert.equal(refund?.providerReference, `simulated_re_${result.actionId}`);
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
  approvals = 0;
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

  financialOperation<Input, Facts>(
    spec: FinancialOperationSpec<Input, Facts>,
  ): FinancialOperation<Input, Facts> {
    return {
      buildRequest: (input, facts, options) => {
        const factsValue = facts as Facts;
        const action: CreateFinancialActionRequest['action'] = {
          kind: spec.kind,
          operation: spec.operation,
          principal_id: spec.principalId,
          amount: spec.amount(input, factsValue),
          rail: spec.rail,
          metadata: spec.metadata?.(input, factsValue) ?? {},
        };
        const counterparty = spec.counterparty?.(input, factsValue);
        if (counterparty !== undefined) action.counterparty = counterparty;
        const mandate = spec.mandate?.(input, factsValue);
        if (mandate !== undefined) action.mandate = mandate;
        const memo = spec.memo?.(input, factsValue);
        if (memo !== undefined) action.memo = memo;

        return {
          idempotency_key: spec.idempotencyKey(input, factsValue),
          execute: options?.execute ?? spec.execute ?? false,
          action,
          evidence: spec.evidence?.(input, factsValue) ?? [],
        };
      },
      verify: async (input, facts, options) => {
        const request = this.financialOperation(spec).buildRequest(input, facts, options);
        return this.guardPayment(request);
      },
    };
  }

  async getFinancialAction(actionId: string): Promise<FinancialActionRecord> {
    return this.requireAction(actionId);
  }

  async approveAction(actionId: string): Promise<FinancialActionRecord> {
    const current = this.requireAction(actionId);
    if (current.status !== 'proposed') return current;
    this.approvals += 1;
    const authorized: FinancialActionRecord = {
      ...current,
      status: 'authorized',
      updated_at: timestamp(),
    };
    this.actions.set(actionId, authorized);
    return authorized;
  }

  async executeAction(actionId: string): Promise<FinancialActionRecord> {
    const current = this.requireAction(actionId);
    if (current.status !== 'authorized') return current;
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
        provider: {
          reference: `simulated_re_${actionId}`,
          status: 'succeeded',
        },
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

function customerRefunds() {
  return customerBackendState().refunds;
}

function timestamp(): string {
  return '2026-07-06T10:00:00.000Z';
}

await testOrderSearch();
await testPrepareRefundBuildsTypedAction();
await testOverRefundStillSubmitsFinancialAction();
await testOfflineAgentApprovesAndExecutesProposedRefund();
await testHeldActionDoesNotExecute();
await testProviderAuthAndSimulation();
await testStripeSafetyAndMapping();

process.stdout.write('stripe-refund-agent checks passed\n');
