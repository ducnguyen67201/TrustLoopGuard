import assert from 'node:assert/strict';

import { buildRefundRequest, createRefundGrant, REFUND_SCENARIOS, runRefundDemo } from './core';
import { MockFinancialRefundClient } from './mock-client';

async function main(): Promise<void> {
  const client = new MockFinancialRefundClient();
  const rows = await runRefundDemo({ client, providerCallCount: () => client.providerCallCount() });
  const byKey = Object.fromEntries(rows.map((row) => [row.key, row]));
  assert.equal(rows.length, REFUND_SCENARIOS.length);
  assert.equal(byKey.saved_grant?.authorizationEffect, 'permit');
  assert.equal(byKey.saved_grant?.executionStatus, 'succeeded');
  assert.equal(byKey.approval_threshold?.authorizationEffect, 'permit');
  assert.equal(byKey.approval_threshold?.executionStatus, 'succeeded');
  assert.equal(byKey.missing_grant?.authorizationEffect, 'require_approval');
  assert.equal(byKey.missing_grant?.executionStatus, 'not_started');
  assert.equal(byKey.missing_grant?.providerCalls, 0);
  assert.equal(byKey.duplicate_idempotency?.duplicateReusedAction, true);
  assert.equal(byKey.duplicate_idempotency?.providerCalls, 1);

  const requestClient = new MockFinancialRefundClient();
  const grant = await createRefundGrant(requestClient);
  const request = buildRefundRequest(REFUND_SCENARIOS[0]!, grant);
  assert.equal(request.authorization?.grant_id, grant.id);
  assert.equal(request.action.amount.amount_minor, 4_000n);
  process.stdout.write('financial refund demo check: all assertions passed\n');
}

main().catch((error) => {
  process.stderr.write(`financial refund demo check failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
