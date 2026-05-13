// Smallest possible TrustLoopGuard integration in TypeScript.
//
// Run a local tl-server and then:
//
//     pnpm --filter @trustloopguard/example-typescript start \
//       "show me my password" "here it is: hunter2"
//
// Imports only @trustloopguard/sdk. The SDK owns the guardrail request;
// application code only guards the output boundary.

import { guard, type GuardLogEvent } from '@trustloopguard/sdk';

async function main(): Promise<number> {
  const args = process.argv.slice(2);
  const input = args[0] ?? 'hello';
  const draft = args[1] ?? 'hi there';

  const events: GuardLogEvent[] = [];
  const guardrail = guard({
    agentId: 'example-typescript',
    log: (next) => {
      events.push(next);
    },
  });

  const reply = await guardrail({ input, draft });
  const event = events.at(-1);

  console.log(`reply         : ${reply}`);
  if (event !== undefined) {
    console.log(`verdict       : ${event.verdict}`);
    console.log(`branch        : ${event.branch}`);
    console.log(`trace_id      : ${event.trace_id}`);
    console.log(`latency_ms    : ${event.latency_ms}`);
  }

  if (event?.branch === 'block' || event?.branch === 'escalate') {
    return 2;
  }
  if (event?.branch === 'error') {
    return 1;
  }
  return 0;
}

main().then(
  (code) => process.exit(code),
  (err) => {
    console.error(err);
    process.exit(1);
  },
);
