import { runRefundDemo, type ScenarioResult } from './core';
import { MockFinancialRefundClient } from './mock-client';

function printTable(rows: ScenarioResult[]): void {
  const line = '-'.repeat(112);
  process.stdout.write(`${line}\n`);
  process.stdout.write(
    ` #  ${'scenario'.padEnd(42)}${'initial'.padEnd(12)}${'final'.padEnd(12)}${'provider'.padEnd(10)}${'receipt'.padEnd(10)}outcome\n`,
  );
  rows.forEach((row, i) => {
    process.stdout.write(
      ` ${String(i + 1).padEnd(3)}${row.label.padEnd(42)}${row.initialStatus.padEnd(12)}${row.finalStatus.padEnd(12)}${String(row.providerCalls).padEnd(10)}${String(row.receiptExported).padEnd(10)}${row.outcomeRecorded}\n`,
    );
  });
  process.stdout.write(`${line}\n`);
  const executed = rows.filter((row) => row.finalStatus === 'executed').length;
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
