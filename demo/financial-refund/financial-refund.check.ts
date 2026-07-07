import assert from 'node:assert/strict';

import { buildRefundRequest, createRefundMandate, REFUND_SCENARIOS, runRefundDemo } from './core';
import { MockFinancialRefundClient } from './mock-client';

async function main(): Promise<void> {
  const client = new MockFinancialRefundClient();
  const rows = await runRefundDemo({
    client,
    providerCallCount: () => client.providerCallCount(),
  });

  const byKey = Object.fromEntries(rows.map((row) => [row.key, row]));

  assert.equal(rows.length, REFUND_SCENARIOS.length, 'all refund scenarios ran');

  assert.equal(byKey.normal_allow?.initialStatus, 'executed');
  assert.equal(byKey.normal_allow?.finalStatus, 'executed');
  assert.equal(byKey.normal_allow?.providerCalls, 1);
  assert.equal(byKey.normal_allow?.receiptExported, true);
  assert.equal(byKey.normal_allow?.outcomeRecorded, true);

  assert.equal(byKey.hold_then_approve?.initialStatus, 'held');
  assert.equal(byKey.hold_then_approve?.finalStatus, 'executed');
  assert.equal(byKey.hold_then_approve?.providerCalls, 1);
  assert.equal(byKey.hold_then_approve?.receiptExported, true);

  assert.equal(byKey.hold_then_deny?.initialStatus, 'held');
  assert.equal(byKey.hold_then_deny?.finalStatus, 'denied');
  assert.equal(byKey.hold_then_deny?.providerCalls, 0);
  assert.equal(byKey.hold_then_deny?.receiptExported, false);

  assert.equal(byKey.duplicate_idempotency?.finalStatus, 'executed');
  assert.equal(byKey.duplicate_idempotency?.duplicateReusedAction, true);
  assert.equal(byKey.duplicate_idempotency?.providerCalls, 1);

  assert.equal(byKey.missing_mandate?.initialStatus, 'denied');
  assert.equal(byKey.missing_mandate?.finalStatus, 'denied');
  assert.equal(byKey.missing_mandate?.providerCalls, 0);

  const requestClient = new MockFinancialRefundClient();
  const mandate = await createRefundMandate(requestClient);
  const request = buildRefundRequest(REFUND_SCENARIOS[0]!, mandate);
  assert.equal(request.action.kind, 'refund');
  assert.equal(request.action.amount.amount_minor, 4_000n);
  assert.equal(request.action.metadata?.order_id, 'order_123');
  assert.equal(request.evidence[0]?.kind, 'refund_eligibility');
  assert.equal(request.evidence[0]?.metadata?.payment_captured, true);

  process.stdout.write('financial refund demo check: all assertions passed\n');
}

main().catch((error) => {
  process.stderr.write(
    `financial refund demo check failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
