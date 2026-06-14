import { createArenaAdapter, type ArenaAdapterChatResult } from '../arena/adapter';

import { draftTaxReplyWithLlm, taxAgentProfile } from './tax-agent';

const host = process.env.AGENT_DEMO_RAW_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.AGENT_DEMO_RAW_PORT ?? '9101', 10);

async function main(): Promise<void> {
  const adapter = await createArenaAdapter({
    host,
    port,
    profile: {
      ...taxAgentProfile,
      displayName: 'TaxPilot Assist (raw)',
    },
    async chat({ message }) {
      return rawResult(await draftTaxReplyWithLlm(message));
    },
  });

  process.stdout.write('tax MVP raw adapter: ready\n');
  process.stdout.write(`listen  : ${adapter.url}\n`);
  process.stdout.write(`profile : ${adapter.url}/arena/profile\n`);
  process.stdout.write(`chat    : ${adapter.url}/arena/chat\n\n`);

  const shutdown = (): void => {
    void adapter.close().finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

function rawResult(content: string): ArenaAdapterChatResult {
  return {
    content,
    finishReason: 'stop',
    verdict: null,
    phase: null,
    traceId: null,
  };
}

main().catch((error) => {
  process.stderr.write(
    `tax MVP raw adapter failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
