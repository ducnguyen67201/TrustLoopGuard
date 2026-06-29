import assert from 'node:assert/strict';

import type { Decision } from '@trustloopguard/sdk';

import type { PayFn, SubmitFn } from './scenarios.core';
import { buildEvent, runScenarios, SCENARIOS } from './scenarios.core';

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

async function main(): Promise<void> {
  // 1) Each scenario builds the expected event shape.
  for (const scenario of SCENARIOS) {
    const event = buildEvent(scenario, 'agent-test');
    const destParam = scenario.tool === 'issue_refund' ? 'account' : 'destination';
    const params = event.action.parameters;

    assert.equal(event.kind, 'tool.call.proposed');
    assert.equal(event.action.operation, scenario.tool);
    assert.notEqual(params, null, `${scenario.label}: parameters present`);
    assert.ok(params !== null && destParam in params, `${scenario.label}: has ${destParam}`);

    const amount = params === null ? undefined : params.amount;
    if (scenario.ambiguousAmount === true) {
      assert.ok(!Number.isInteger(amount), `${scenario.label}: ambiguous amount is non-integer`);
    } else {
      assert.ok(Number.isInteger(amount), `${scenario.label}: amount is integer cents`);
    }

    const expectedSource = scenario.destinationTrusted ? 'account_registry' : 'conversation';
    assert.deepEqual(event.provenance?.[destParam], [expectedSource]);
    assert.equal(event.sources?.[0]?.id, expectedSource);
  }

  // 2) The core guarantee: a payment fires ONLY on `allow`.
  let payCalls = 0;
  const spyPay: PayFn = async () => {
    payCalls += 1;
    return { ref: '[spy]', mode: 'simulated' };
  };

  const blockAll: SubmitFn = async () => makeDecision('block', 'parameter_value.amount');
  const blockedRows = await runScenarios({ agentId: 'a', submit: blockAll, pay: spyPay });
  assert.equal(payCalls, 0, 'no payment is executed when blocked');
  assert.ok(blockedRows.every((row) => row.result === 'stopped before payment'));
  assert.ok(blockedRows.every((row) => row.control === 'value_limit'));

  payCalls = 0;
  const allowAll: SubmitFn = async () => makeDecision('allow');
  const allowedRows = await runScenarios({ agentId: 'a', submit: allowAll, pay: spyPay });
  assert.equal(payCalls, SCENARIOS.length, 'a payment is executed on every allow');
  assert.ok(allowedRows.every((row) => row.control === 'none'));

  // 3) The printed control is read from the decision's own evidence.
  const escalateRows = await runScenarios({
    agentId: 'a',
    submit: async () => makeDecision('escalate', 'approval.wire_transfer'),
    pay: spyPay,
  });
  assert.equal(escalateRows[0]?.verdict, 'escalate');
  assert.equal(escalateRows[0]?.control, 'approval');

  process.stdout.write('scenarios check: all assertions passed\n');
}

main().catch((error) => {
  process.stderr.write(`scenarios check failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
