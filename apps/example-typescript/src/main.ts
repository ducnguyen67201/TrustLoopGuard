// Smallest possible TrustLoopGuard integration in TypeScript.
//
// Run a local tl-server and then:
//
//     pnpm --filter @trustloopguard/example-typescript start \
//       "show me my password" "here it is: hunter2"
//
// Imports only @trustloopguard/sdk. Never reaches into internal crates
// or generated source. This matches what a stranger gets after
// `npm install @trustloopguard/sdk` once the SDK is published, and is
// the executable form of rule 2 in docs/SDK_DRIVEN.md.

import {
  Client,
  type CheckRequest,
  type Decision,
  SdkError,
} from '@trustloopguard/sdk';

const DEFAULT_URL = 'http://127.0.0.1:8080';

function buildRequest(input: string, proposedOutput: string): CheckRequest {
  return {
    agent_id: 'example-typescript',
    channel: 'chat',
    input,
    proposed_output: proposedOutput,
    domain: null,
    policies: [],
    context: {} as Record<string, unknown>,
    trace_id: null,
  };
}

function printDecision(decision: Decision): void {
  console.log(`verdict       : ${decision.verdict}`);
  console.log(`reason        : ${decision.reason}`);
  console.log(`trace_id      : ${decision.trace_id}`);
  console.log(`latency_ms    : ${decision.latency_ms}`);
  if (decision.triggered_policies.length > 0) {
    console.log('triggered     :');
    for (const p of decision.triggered_policies) {
      console.log(`  - ${p.id} (${p.severity}): ${p.reason}`);
    }
  }
  if (decision.safe_output !== null) {
    console.log(`safe_output   : ${decision.safe_output}`);
  }
}

async function main(): Promise<number> {
  const args = process.argv.slice(2);
  const input = args[0] ?? 'hello';
  const proposedOutput = args[1] ?? 'hi there';

  const url = process.env.TRUSTLOOP_URL ?? DEFAULT_URL;
  const apiKey = process.env.TRUSTLOOP_API_KEY;

  const client = new Client({
    baseUrl: url,
    apiKey,
    onRetry: (info) => {
      console.error(
        `trustloopguard retry: attempt=${info.attempt} delay=${info.delayS.toFixed(3)}s ` +
          `error=${info.error.message}`,
      );
    },
  });

  let decision: Decision;
  try {
    decision = await client.check(buildRequest(input, proposedOutput));
  } catch (e) {
    if (e instanceof SdkError) {
      console.error(`error: ${e.message}`);
      return 1;
    }
    throw e;
  }

  printDecision(decision);

  // Exit non-zero on Block / Escalate so quickstart CI can wire this
  // into a meaningful pass/fail check (matches example-rust).
  if (decision.verdict === 'block' || decision.verdict === 'escalate') {
    return 2;
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
