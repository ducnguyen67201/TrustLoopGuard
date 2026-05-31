import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';

export type ArenaJsonValue =
  | string
  | number
  | boolean
  | null
  | ArenaJsonValue[]
  | { [key: string]: ArenaJsonValue };

export interface ArenaAdapterProfile {
  displayName: string;
  surface: 'chat';
  systemPrompt: string;
  safeUserQuestion: string;
  protectedInformationName: string;
  model?: string;
}

export interface ArenaAdapterChatRequest {
  message: string;
}

export type ArenaAdapterFinishReason = 'stop' | 'content_filter';
export type ArenaAdapterVerdict = 'blocked' | 'escalated' | null;
export type ArenaAdapterPhase = 'input' | 'output' | null;

export interface ArenaAdapterChatResult {
  content: string;
  finishReason: ArenaAdapterFinishReason;
  verdict: ArenaAdapterVerdict;
  phase: ArenaAdapterPhase;
  traceId: string | null;
}

export interface ArenaAdapterServer {
  url: string;
  close: () => Promise<void>;
}

export interface CreateArenaAdapterOptions {
  host: string;
  port: number;
  profile: ArenaAdapterProfile;
  chat: (request: ArenaAdapterChatRequest) => Promise<ArenaAdapterChatResult>;
}

export async function createArenaAdapter({
  host,
  port,
  profile,
  chat,
}: CreateArenaAdapterOptions): Promise<ArenaAdapterServer> {
  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    if (handleCors(req, res)) return;

    void handleRequest(req, res, profile, chat).catch((error) => {
      writeJson(res, 500, { error: error instanceof Error ? error.message : String(error) });
    });
  });

  await listen(server, host, port);

  return {
    url: `http://${host}:${port}`,
    close: () => closeServer(server),
  };
}

async function handleRequest(
  req: IncomingMessage,
  res: ServerResponse,
  profile: ArenaAdapterProfile,
  chat: CreateArenaAdapterOptions['chat'],
): Promise<void> {
  if (req.method === 'GET' && req.url === '/health') {
    writeJson(res, 200, { ok: true });
    return;
  }

  if (req.method === 'GET' && req.url === '/arena/profile') {
    writeJson(res, 200, profile);
    return;
  }

  if (req.method === 'POST' && req.url === '/arena/chat') {
    const body = await readJsonRequest(req);
    const request = parseChatRequest(body);
    if (request === null) {
      writeJson(res, 400, { error: 'expected JSON body with non-empty string field `message`' });
      return;
    }

    const startedAt = Date.now();
    const result = await chat(request);
    writeJson(res, 200, {
      agent: profile.displayName,
      ...result,
      latencyMs: Date.now() - startedAt,
    });
    return;
  }

  writeJson(res, 404, { error: 'not found' });
}

function parseChatRequest(body: ArenaJsonValue): ArenaAdapterChatRequest | null {
  if (!isJsonObject(body) || typeof body.message !== 'string' || body.message.trim() === '') {
    return null;
  }

  return { message: body.message };
}

async function readJsonRequest(req: IncomingMessage): Promise<ArenaJsonValue> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }

  const text = Buffer.concat(chunks).toString('utf8');
  return text ? (JSON.parse(text) as ArenaJsonValue) : {};
}

function writeJson(res: ServerResponse, status: number, body: object): void {
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

function isJsonObject(value: ArenaJsonValue): value is { [key: string]: ArenaJsonValue } {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

async function listen(
  server: ReturnType<typeof createServer>,
  host: string,
  port: number,
): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, host, () => {
      server.off('error', reject);
      resolve();
    });
  });
}

async function closeServer(server: ReturnType<typeof createServer>): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.close((error) => {
      if (error) reject(error);
      else resolve();
    });
  });
}
