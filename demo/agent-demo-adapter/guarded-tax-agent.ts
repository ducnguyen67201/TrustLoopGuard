import { guard, type GuardLogEvent } from '@trustloopguard/sdk';

import { createArenaAdapter, type ArenaAdapterChatResult } from '../arena/adapter';
import { createClient, DEFAULT_AGENT_ID, SERVER_URL, WORKSPACE_ID } from '../shared/env';

import { draftTaxReply, taxAgentProfile } from './tax-agent';

const host = process.env.AGENT_DEMO_GUARDED_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.AGENT_DEMO_GUARDED_PORT ?? '9102', 10);
const agentId = process.env.TL_AGENT_ID ?? DEFAULT_AGENT_ID;

async function main(): Promise<void> {
  const client = createClient();
  const adapter = await createArenaAdapter({
    host,
    port,
    profile: {
      ...taxAgentProfile,
      displayName: 'TaxPilot Assist (guarded)',
    },
    async chat({ message }) {
      let event: GuardLogEvent | null = null;
      const draft = draftTaxReply(message);
      const content = await guard({
        client,
        agentId,
        channel: 'chat',
        input: message,
        draft,
        context: {
          demo_surface: 'tax_mvp_chat',
          product: 'TaxPilot Assist',
          data_domain: 'tax_preparation',
          adapter: 'attacks_tab',
          tax_packet_source: 'local_fixture_until_workspace_provider_exists',
        },
        onBlock: (decision) =>
          decision.safe_output ??
          'I cannot share or change protected tax-packet details in chat. A preparer can review this request.',
        onEscalate: () =>
          'This tax-packet request needs preparer review before I can answer in chat.',
        onRevise: (revised: string | null | undefined, original: string) => revised ?? original,
        onError: (_error, original) => original,
        log: (nextEvent) => {
          event = nextEvent;
        },
      });

      return guardedResult(content, event);
    },
  });

  process.stdout.write('tax MVP guarded adapter: ready\n');
  process.stdout.write(`listen    : ${adapter.url}\n`);
  process.stdout.write(`profile   : ${adapter.url}/arena/profile\n`);
  process.stdout.write(`chat      : ${adapter.url}/arena/chat\n`);
  process.stdout.write(`server    : ${SERVER_URL}\n`);
  process.stdout.write(`workspace : ${WORKSPACE_ID ?? '(default runtime workspace)'}\n`);
  process.stdout.write(`agent     : ${agentId}\n\n`);

  const shutdown = (): void => {
    void adapter.close().finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

function guardedResult(content: string, event: GuardLogEvent | null): ArenaAdapterChatResult {
  if (event?.verdict === 'block') {
    return {
      content,
      finishReason: 'content_filter',
      verdict: 'blocked',
      phase: 'output',
      traceId: event.trace_id ?? null,
    };
  }

  if (event?.verdict === 'escalate') {
    return {
      content,
      finishReason: 'content_filter',
      verdict: 'escalated',
      phase: 'output',
      traceId: event.trace_id ?? null,
    };
  }

  return {
    content,
    finishReason: 'stop',
    verdict: null,
    phase: null,
    traceId: event?.trace_id ?? null,
  };
}

main().catch((error) => {
  process.stderr.write(
    `tax MVP guarded adapter failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
