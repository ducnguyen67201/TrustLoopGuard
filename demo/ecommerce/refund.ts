import { createClient, DEFAULT_AGENT_ID } from '../shared/env';

import { runRefundPilot, type RefundPilotRow } from './refund.core';

function printTable(rows: RefundPilotRow[]): void {
  const line = '-'.repeat(88);
  process.stdout.write(`${line}\n`);
  process.stdout.write(` #  ${'scenario'.padEnd(34)}${'verdict'.padEnd(10)}${'control'.padEnd(15)}result\n`);
  rows.forEach((row, i) => {
    process.stdout.write(
      ` ${String(i + 1).padEnd(3)}${row.label.padEnd(34)}${row.verdict.padEnd(10)}${row.control.padEnd(15)}${row.result}\n`,
    );
  });
  process.stdout.write(`${line}\n`);
  const issued = rows.filter((row) => row.verdict === 'allow').length;
  process.stdout.write(`${issued} simulated side effect(s) executed, ${rows.length - issued} stopped before refund.\n`);
}

function assertEnforced(rows: RefundPilotRow[]): void {
  if (rows.every((row) => row.verdict === 'allow')) {
    process.stderr.write(
      '\nEvery refund scenario was ALLOWED - checker enforcement is OFF for this workspace.\n' +
        'Run ecommerce:setup with TL_USER_ID set to a workspace owner, or enable param and approval checkers in Settings.\n',
    );
    process.exitCode = 1;
  }
}

async function main(): Promise<void> {
  const client = createClient();
  const agentId = process.env.TL_AGENT_ID ?? DEFAULT_AGENT_ID;
  process.stdout.write('\nE-commerce Refund Pilot - guarded run (simulated side effects only)\n');

  const rows = await runRefundPilot({
    agentId,
    submit: (event) => client.submitEvent(event),
  });
  printTable(rows);
  assertEnforced(rows);
}

main().catch((error) => {
  process.stderr.write(`ecommerce refund pilot failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
});
