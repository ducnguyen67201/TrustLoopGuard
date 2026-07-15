import { runRefundDemo, type ScenarioResult } from './core';
import { MockFinancialRefundClient } from './mock-client';

function printTable(rows: ScenarioResult[]): void {
  const line = '-'.repeat(112);
  process.stdout.write(`${line}\n`);
  process.stdout.write(
    ` #  ${'scenario'.padEnd(52)}${'effect'.padEnd(20)}${'execution'.padEnd(14)}${'provider'.padEnd(10)}receipt\n`,
  );
  rows.forEach((row, i) => {
    process.stdout.write(
      ` ${String(i + 1).padEnd(3)}${row.label.padEnd(52)}${row.authorizationEffect.padEnd(20)}${row.executionStatus.padEnd(14)}${String(row.providerCalls).padEnd(10)}${row.executionReceiptExported}\n`,
    );
  });
  process.stdout.write(`${line}\n`);
  const executed = rows.filter((row) => row.executionStatus === 'succeeded').length;
  const stopped = rows.length - executed;
  process.stdout.write(
    `${executed} authorized refund(s) executed, ${stopped} stopped or held before provider execution.\n`,
  );
}

async function main(): Promise<void> {
  const client = new MockFinancialRefundClient();
  process.stdout.write('\nAgentic refund authorization demo (offline mock provider)\n');
  const rows = await runRefundDemo({
    client,
    providerCallCount: () => client.providerCallCount(),
  });
  printTable(rows);
}

main().catch((error) => {
  process.stderr.write(
    `financial refund demo failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
