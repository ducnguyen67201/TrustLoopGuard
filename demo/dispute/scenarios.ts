// Flagship money-agent showcase: a fixed set of money-move attempts, each
// isolating ONE control, run through the guard. A payment fires only on
// `allow`. Run with `pnpm --filter @trustloopguard/demo dispute:scenarios`.
//
// The pure scenario logic lives in scenarios.core.ts (so tests can import it
// without triggering a run); this file is the runnable entry: real client,
// real payment executor, table output, and a loud guard against misconfig.

import { createClient, DEFAULT_AGENT_ID } from '../shared/env';

import { executePayment } from './payments';
import { formatScenarioTranscript, runScenarios, type ScenarioRow } from './scenarios.core';

/** If nothing was stopped, the workspace's checkers are off — fail loudly. */
function assertEnforced(rows: ScenarioRow[]): void {
  if (rows.every((row) => row.verdict === 'allow')) {
    process.stderr.write(
      '\n⚠ Every scenario was ALLOWED — checker enforcement is OFF for this workspace.\n' +
        '  Run: TL_USER_ID=<owner-uuid> pnpm --filter @trustloopguard/demo dispute:setup\n' +
        '  Or set param_checker_mode and approval_checker_mode to "enforce" in Settings, then re-run.\n',
    );
    process.exitCode = 1;
  }
}

async function main(): Promise<void> {
  const client = createClient();
  const agentId = process.env.TL_AGENT_ID ?? DEFAULT_AGENT_ID;
  const paymentMode = (process.env.STRIPE_SECRET_KEY ?? '').trim() !== '' ? 'stripe-test' : 'simulated';

  const rows = await runScenarios({
    agentId,
    submit: (event) => client.submitEvent(event),
    pay: executePayment,
  });
  process.stdout.write(formatScenarioTranscript(rows, { paymentMode }));
  assertEnforced(rows);
}

main().catch((error) => {
  process.stderr.write(`dispute scenarios failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
});
