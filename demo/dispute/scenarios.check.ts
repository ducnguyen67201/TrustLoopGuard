import assert from 'node:assert/strict';

import type { AuthorizationDecision, AuthorizationEffect } from '@featherlane-ai/sdk';

import type { PayFn, SubmitFn } from './scenarios.core';
import { buildEvent, runScenarios, SCENARIOS } from './scenarios.core';

function makeDecision(effect: AuthorizationEffect, findingId?: string): AuthorizationDecision {
  return {
    trace_id: 'trace-test',
    domain: 'tool',
    effect,
    reason: 'test',
    findings: findingId === undefined ? [] : [{
      id: findingId, source: 'test', effect, reason: 'test', severity: 'high', evidence: {},
    }],
    latency_ms: 0n,
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

  // 2) The core guarantee: a payment fires ONLY on `permit`.
  let payCalls = 0;
  const spyPay: PayFn = async () => {
    payCalls += 1;
    return { ref: '[spy]', mode: 'simulated' };
  };

  const blockAll: SubmitFn = async () => makeDecision('deny', 'parameter_value.amount');
  const blockedRows = await runScenarios({ agentId: 'a', submit: blockAll, pay: spyPay });
  assert.equal(payCalls, 0, 'no payment is executed when blocked');
  assert.ok(blockedRows.every((row) => row.result === 'stopped before payment'));
  assert.ok(blockedRows.every((row) => row.control === 'value_limit'));

  payCalls = 0;
  const allowAll: SubmitFn = async () => makeDecision('permit');
  const allowedRows = await runScenarios({ agentId: 'a', submit: allowAll, pay: spyPay });
  assert.equal(payCalls, SCENARIOS.length, 'a payment is executed on every permit');
  assert.ok(allowedRows.every((row) => row.control === 'none'));

  // 3) The printed control is read from the decision's own evidence.
  const approvalRows = await runScenarios({
    agentId: 'a',
    submit: async () => makeDecision('require_approval', 'approval.wire_transfer'),
    pay: spyPay,
  });
  assert.equal(approvalRows[0]?.effect, 'require_approval');
  assert.equal(approvalRows[0]?.control, 'approval');

  process.stdout.write('scenarios check: all assertions passed\n');
}

main().catch((error) => {
  process.stderr.write(`scenarios check failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
