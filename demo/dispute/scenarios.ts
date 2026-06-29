// Flagship money-agent showcase: a fixed set of money-move attempts, each
// isolating ONE control, run through the guard. A payment fires only on
// `allow`. Run with `pnpm --filter @trustloopguard/demo dispute:scenarios`.
//
// The pure scenario logic lives in scenarios.core.ts (so tests can import it
// without triggering a run); this file is the runnable entry: real client,
// real payment executor, table output, and a loud guard against misconfig.

import { createClient, DEFAULT_AGENT_ID } from '../shared/env';

import { executePayment } from './payments';
import { runScenarios, type ScenarioRow } from './scenarios.core';

function printTable(rows: ScenarioRow[]): void {
  const line = '─'.repeat(78);
  process.stdout.write(`${line}\n`);
  process.stdout.write(` #  ${'scenario'.padEnd(34)}${'verdict'.padEnd(10)}${'control'.padEnd(15)}result\n`);
  rows.forEach((row, i) => {
    process.stdout.write(
      ` ${String(i + 1).padEnd(3)}${row.label.padEnd(34)}${row.verdict.padEnd(10)}${row.control.padEnd(15)}${row.result}\n`,
    );
  });
  process.stdout.write(`${line}\n`);
  const paid = rows.filter((row) => row.verdict === 'allow').length;
  process.stdout.write(
    `${paid} payment(s) executed, ${rows.length - paid} stopped — every stop happened BEFORE money moved.\n`,
  );
}

/** If nothing was stopped, the workspace's checkers are off — fail loudly. */
function assertEnforced(rows: ScenarioRow[]): void {
  if (rows.every((row) => row.verdict === 'allow')) {
    process.stderr.write(
      '\n⚠ Every scenario was ALLOWED — checker enforcement is OFF for this workspace.\n' +
        '  Run setup with TL_USER_ID set to a workspace owner, or set param_checker_mode and\n' +
        '  approval_checker_mode to "enforce" in Settings, then re-run.\n',
    );
    process.exitCode = 1;
  }
}

async function main(): Promise<void> {
  const client = createClient();
  const agentId = process.env.TL_AGENT_ID ?? DEFAULT_AGENT_ID;
  const stripe = (process.env.STRIPE_SECRET_KEY ?? '').trim() !== '';
  process.stdout.write(`\nMoney Agent — guarded run  (payments: ${stripe ? 'stripe-test' : 'simulated'})\n`);

  const rows = await runScenarios({
    agentId,
    submit: (event) => client.submitEvent(event),
    pay: executePayment,
  });
  printTable(rows);
  assertEnforced(rows);
}

main().catch((error) => {
  process.stderr.write(`dispute scenarios failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
});
