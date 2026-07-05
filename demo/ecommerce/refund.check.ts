import assert from 'node:assert/strict';

import type { Decision } from '@trustloopguard/sdk';

import type { RefundLedger, SubmitFn } from './refund.core';
import { buildRefundEvent, controlFor, runRefundPilot, SCENARIOS } from './refund.core';

function makeDecision(verdict: Decision['verdict'], violatedRule?: string): Decision {
  return {
    trace_id: 'trace-test',
    verdict,
    reason: 'test',
    triggered_policies: [],
    safe_output: null,
    latency_ms: 0n,
    tier_results: [],
    redaction: null,
    violated_rule: violatedRule,
  };
}

function decisionForControl(control: string): Decision {
  if (control === 'value_limit') return makeDecision('block', 'parameter_value.amount');
  if (control === 'parameter_auth') return makeDecision('block', 'parameter_source.destination');
  if (control === 'approval') return makeDecision('escalate', 'approval.ecommerce_issue_store_credit');
  return makeDecision('allow');
}

async function main(): Promise<void> {
  assert.equal(SCENARIOS.length, 5, 'pilot has exactly five focused scenarios');
  assert.deepEqual(
    SCENARIOS.map((scenario) => scenario.expectedControl),
    ['none', 'value_limit', 'parameter_auth', 'value_limit', 'approval'],
    'each scenario has exactly one intended control',
  );

  for (const scenario of SCENARIOS) {
    const event = buildRefundEvent(scenario, 'agent-test');
    const params = event.action.parameters;

    assert.equal(event.kind, 'tool.call.proposed');
    assert.equal(event.action.operation, scenario.tool);
    assert.notEqual(params, null, `${scenario.label}: parameters present`);
    assert.equal(event.context.product, 'E-commerce Refund Pilot');
    assert.equal(event.context.domain, 'ecommerce');
    assert.equal(params?.order_id, scenario.orderId);
    assert.equal(params?.customer_id, scenario.customerId);
    assert.equal(params?.refund_method, scenario.refundMethod);

    if (scenario.ambiguousAmount === true) {
      assert.ok(!Number.isInteger(params?.amount), `${scenario.label}: ambiguous amount is non-integer cents`);
    } else {
      assert.ok(Number.isInteger(params?.amount), `${scenario.label}: amount is integer cents`);
    }

    const expectedDestinationSource = scenario.destinationTrusted ? 'order_registry' : 'conversation';
    assert.deepEqual(event.provenance?.amount, ['order_registry']);
    assert.deepEqual(event.provenance?.destination, [expectedDestinationSource]);
    assert.equal(event.sources?.some((source) => source.id === expectedDestinationSource), true);
  }

  const ledger: RefundLedger = [];
  const submit: SubmitFn = async (event) => {
    const scenario = SCENARIOS.find((candidate) => candidate.tool === event.action.operation);
    assert.ok(scenario, 'scenario can be matched from event');
    return decisionForControl(scenario.expectedControl);
  };

  const rows = await runRefundPilot({ agentId: 'agent-test', submit, ledger });
  assert.equal(ledger.length, 1, 'only the allowed refund writes to the ledger');
  assert.equal(ledger[0]?.kind, 'refund');
  assert.equal(ledger[0]?.amountCents, 5_000);
  assert.ok(rows.some((row) => row.verdict === 'allow' && row.result === 'refund issued (simulated)'));
  assert.ok(rows.filter((row) => row.verdict !== 'allow').every((row) => row.result === 'stopped before refund'));

  const approvalDecision = makeDecision('escalate', 'approval.ecommerce_issue_store_credit');
  assert.equal(controlFor(approvalDecision), 'approval');

  process.stdout.write('ecommerce refund check: all assertions passed\n');
}

main().catch((error) => {
  process.stderr.write(`ecommerce refund check failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
