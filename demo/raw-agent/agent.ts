import { createArenaAdapter, type ArenaAdapterServer } from '../arena/adapter';
import { chatBreakCases, mockProviderReplyFor, proxySupportAgent } from '../proxy/config';

const host = process.env.RAW_AGENT_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.RAW_AGENT_PORT ?? '8787', 10);

async function main(): Promise<void> {
  const adapter = await createArenaAdapter({
    host,
    port,
    profile: proxySupportAgent,
    async chat({ message }) {
      return {
        content: rawReplyFor(message),
        finishReason: 'stop',
        verdict: null,
        phase: null,
        traceId: null,
      };
    },
  });

  printReady(adapter);

  const shutdown = (): void => {
    void adapter.close().finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

function rawReplyFor(userMessage: string): string {
  const breakCase = chatBreakCases.find((candidate) => candidate.userMessage === userMessage);
  return breakCase ? mockProviderReplyFor(breakCase) : "I don't have a scripted answer for that prompt.";
}

function printReady(adapter: ArenaAdapterServer): void {
  process.stdout.write('raw agent: ready\n');
  process.stdout.write(`agent   : ${proxySupportAgent.displayName}\n`);
  process.stdout.write(`listen  : ${adapter.url}\n`);
  process.stdout.write(`profile : ${adapter.url}/arena/profile\n`);
  process.stdout.write(`chat    : ${adapter.url}/arena/chat\n\n`);
}

main().catch((error) => {
  process.stderr.write(`raw agent failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
