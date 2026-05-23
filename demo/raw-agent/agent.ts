import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';

import { chatBreakCases, mockProviderReplyFor, proxySupportAgent } from '../proxy/config';
import { closeServer, isRecord, readJsonRequest } from '../proxy/runtime';

interface ChatRequest {
  message: string;
}

const host = process.env.RAW_AGENT_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.RAW_AGENT_PORT ?? '8787', 10);

async function main(): Promise<void> {
  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    if (handleCors(req, res)) return;

    void handleRequest(req, res).catch((error: unknown) => {
      writeJson(res, 500, { error: error instanceof Error ? error.message : String(error) });
    });
  });

  await listen(server);
  printReady();

  const shutdown = (): void => {
    void closeServer(server).finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

async function handleRequest(req: IncomingMessage, res: ServerResponse): Promise<void> {
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

    const startedAt = Date.now();
    writeJson(res, 200, {
      agent: proxySupportAgent.displayName,
      content: rawReplyFor(chat.message),
      finishReason: 'stop',
      verdict: null,
      phase: null,
      traceId: null,
      latencyMs: Date.now() - startedAt,
    });
    return;
  }

  writeJson(res, 404, { error: 'not found' });
}

function rawReplyFor(userMessage: string): string {
  const breakCase = chatBreakCases.find((candidate) => candidate.userMessage === userMessage);
  return breakCase ? mockProviderReplyFor(breakCase) : "I don't have a scripted answer for that prompt.";
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

function printReady(): void {
  process.stdout.write('raw agent: ready\n');
  process.stdout.write(`agent   : ${proxySupportAgent.displayName}\n`);
  process.stdout.write(`listen  : http://${host}:${port}\n`);
  process.stdout.write(`profile : http://${host}:${port}/arena/profile\n`);
  process.stdout.write(`chat    : http://${host}:${port}/arena/chat\n\n`);
}

main().catch((error) => {
  process.stderr.write(`raw agent failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
