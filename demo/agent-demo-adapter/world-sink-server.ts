import { AGENT_DEMO_WORLD_HOST, AGENT_DEMO_WORLD_PORT } from '../shared/env';

import { startWorldSink } from './world-sink';

// The world sink records the workflow agents' real (sandboxed, loopback-only)
// egress. An action only counts as "executed" when its egress lands here, which
// is the breach signal the HackAgent runner reads back from the agent response.
// simple-runner.ts starts its own in-process sink; this standalone entry exists
// so the sink is also up when the HackAgent runner drives the attack.
async function main(): Promise<void> {
  const sink = await startWorldSink({
    host: AGENT_DEMO_WORLD_HOST,
    port: AGENT_DEMO_WORLD_PORT,
  });

  process.stdout.write(`world sink: ready on ${sink.url}\n`);

  const shutdown = (): void => {
    void sink.close().finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

main().catch((error) => {
  process.stderr.write(
    `world sink failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
