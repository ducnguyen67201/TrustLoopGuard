import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';

import { proxySupportAgent } from './config';
import {
  closeServer,
  createProxyDemoRuntime,
  isRecord,
  readJsonRequest,
  sendGatewayChatMessage,
  type AgentChatResult,
  type ProxyDemoRuntime,
} from './runtime';

interface ChatRequest {
  message: string;
}

interface ChatResponse extends AgentChatResult {
  agent: string;
}

const host = process.env.PROXY_AGENT_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.PROXY_AGENT_PORT ?? '8788', 10);

async function main(): Promise<void> {
  const runtime = await createProxyDemoRuntime();
  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    if (handleCors(req, res)) return;

    void handleRequest(req, res, runtime).catch((error: unknown) => {
      writeJson(res, 500, { error: error instanceof Error ? error.message : String(error) });
    });
  });

  await listen(server);
  printReady(runtime);

  const shutdown = (): void => {
    void Promise.all([closeServer(server), runtime.close()]).finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

async function handleRequest(
  req: IncomingMessage,
  res: ServerResponse,
  runtime: ProxyDemoRuntime,
): Promise<void> {
  if (req.method === 'GET' && req.url === '/health') {
    writeJson(res, 200, { ok: true });
    return;
  }

  if (req.method === 'GET' && req.url === '/arena/profile') {
    writeJson(res, 200, proxySupportAgent);
    return;
  }

  if (req.method === 'POST' && req.url === '/arena/chat') {
    const body = await readJsonRequest(req);
    const chat = parseChatRequest(body);
    if (!chat) {
      writeJson(res, 400, { error: 'expected JSON body with non-empty string field `message`' });
      return;
    }

    const result = await sendGatewayChatMessage(runtime.route, chat.message);

    writeJson(res, 200, {
      agent: proxySupportAgent.displayName,
      ...result,
    } satisfies ChatResponse);
    return;
  }

  writeJson(res, 404, { error: 'not found' });
}

function parseChatRequest(body: unknown): ChatRequest | null {
  if (!isRecord(body) || typeof body.message !== 'string' || body.message.trim() === '') {
    return null;
  }

  return { message: body.message };
}

function writeJson(res: ServerResponse, status: number, body: unknown): void {
  res.writeHead(status, corsHeaders({ 'content-type': 'application/json' }));
  res.end(JSON.stringify(body));
}

function handleCors(req: IncomingMessage, res: ServerResponse): boolean {
  if (req.method !== 'OPTIONS') return false;

  res.writeHead(204, corsHeaders());
  res.end();
  return true;
}

function corsHeaders(extra: Record<string, string> = {}): Record<string, string> {
  return {
    'access-control-allow-origin': '*',
    'access-control-allow-methods': 'GET,POST,OPTIONS',
    'access-control-allow-headers': 'content-type',
    ...extra,
  };
}

async function listen(server: ReturnType<typeof createServer>): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, host, () => {
      server.off('error', reject);
      resolve();
    });
  });
}

function printReady(runtime: ProxyDemoRuntime): void {
  process.stdout.write('proxy agent: ready\n');
  process.stdout.write(`agent    : ${proxySupportAgent.displayName}\n`);
  process.stdout.write(`listen   : http://${host}:${port}\n`);
  process.stdout.write(`profile  : http://${host}:${port}/arena/profile\n`);
  process.stdout.write(`chat     : http://${host}:${port}/arena/chat\n`);
  process.stdout.write(`workspace: ${runtime.route.workspaceId}\n`);
  process.stdout.write(`route    : ${runtime.gatewayIds.route}\n`);
  process.stdout.write(`provider : ${runtime.route.providerUrl}\n\n`);
}

main().catch((error) => {
  process.stderr.write(`proxy agent failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
