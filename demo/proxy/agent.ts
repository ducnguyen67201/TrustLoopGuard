import {
  createArenaAdapter,
  type ArenaAdapterChatResult,
  type ArenaAdapterServer,
} from '../arena/adapter';
import { proxySupportAgent } from './config';
import {
  createProxyDemoRuntime,
  sendGatewayChatMessage,
  type AgentChatResult,
  type ProxyDemoRuntime,
} from './runtime';

const host = process.env.PROXY_AGENT_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.PROXY_AGENT_PORT ?? '8788', 10);

async function main(): Promise<void> {
  const runtime = await createProxyDemoRuntime();

  let adapter: ArenaAdapterServer;
  try {
    adapter = await createArenaAdapter({
      host,
      port,
      profile: proxySupportAgent,
      async chat({ message }) {
        return gatewayResultToArenaResult(await sendGatewayChatMessage(runtime.route, message));
      },
    });
  } catch (error) {
    await runtime.close();
    throw error;
  }

  printReady(runtime, adapter);

  const shutdown = (): void => {
    void Promise.all([adapter.close(), runtime.close()]).finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

function gatewayResultToArenaResult(result: AgentChatResult): ArenaAdapterChatResult {
  return {
    content: result.content,
    finishReason: result.finishReason === 'content_filter' ? 'content_filter' : 'stop',
    verdict: result.verdict,
    phase: result.phase,
    traceId: result.traceId,
  };
}

function printReady(runtime: ProxyDemoRuntime, adapter: ArenaAdapterServer): void {
  process.stdout.write('proxy agent: ready\n');
  process.stdout.write(`agent    : ${proxySupportAgent.displayName}\n`);
  process.stdout.write(`listen   : ${adapter.url}\n`);
  process.stdout.write(`profile  : ${adapter.url}/arena/profile\n`);
  process.stdout.write(`chat     : ${adapter.url}/arena/chat\n`);
  process.stdout.write(`workspace: ${runtime.route.workspaceId}\n`);
  process.stdout.write(`route    : ${runtime.gatewayIds.route}\n`);
  process.stdout.write(`key      : ${runtime.route.runtimeKey}\n`);
  process.stdout.write(`provider : ${runtime.route.providerUrl}\n\n`);
}

main().catch((error) => {
  process.stderr.write(`proxy agent failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
