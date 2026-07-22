import type { GuardLogEvent } from '@trustloopguard/sdk';

import { createGuardedSupportAgent, draftSupportReply } from './agent';

const input =
  process.argv
    .slice(2)
    .filter((argument) => argument !== '--')
    .join(' ')
    .trim() || 'What is the customer SSN?';
const guardEvents: GuardLogEvent[] = [];
const reply = createGuardedSupportAgent({
  workspaceId: process.env.TL_WORKSPACE_ID,
  log: (event) => guardEvents.push(event),
});

async function main(): Promise<void> {
  const draft = draftSupportReply(input);
  const delivered = await reply(input);
  const event = guardEvents.at(-1);
  const decision = event?.branch === 'error' ? 'error (safe fallback)' : event?.effect;

  process.stdout.write(
    [
      '',
      'TrustLoopGuard output-boundary quickstart',
      `1. User input: ${input}`,
      `2. Agent draft (never deliver this directly): ${draft}`,
      `3. TrustLoopGuard decision: ${decision ?? 'unavailable'}`,
      `   Trace ID: ${event?.trace_id || 'not returned'}`,
      `4. Delivered reply: ${delivered}`,
      '',
    ].join('\n'),
  );
}

main().catch((error) => {
  process.stderr.write(
    `cookbook quickstart failed: ${error instanceof Error ? error.message : String(error)}\n`,
  );
  process.exitCode = 1;
});
