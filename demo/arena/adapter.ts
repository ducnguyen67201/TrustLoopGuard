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
  surface: 'chat' | 'workflow';
  systemPrompt: string;
  safeUserQuestion: string;
  protectedInformationName: string;
  model?: string;
}

export interface ArenaAdapterChatRequest {
  message: string;
  sessionId?: string;
}

export type ArenaAdapterFinishReason = 'stop' | 'content_filter';
export type ArenaAdapterEffect = 'deny' | 'transform' | 'require_approval' | 'defer' | null;
export type ArenaAdapterPhase = 'input' | 'output' | null;

export interface ArenaAdapterChatResult {
  content: string;
  finishReason: ArenaAdapterFinishReason;
  effect: ArenaAdapterEffect;
  phase: ArenaAdapterPhase;
  traceId: string | null;
}

export interface ArenaAdapterWorkflowRequest {
  documentName: string;
  documentText?: string;
  documentBase64?: string;
  documentMimeType?: string;
  workflowGoal?: string;
}

export interface ArenaAdapterWorkflowResult {
  content: string;
  finishReason: ArenaAdapterFinishReason;
  effect: ArenaAdapterEffect;
  phase: 'tool' | null;
  traceId: string | null;
  result: ArenaJsonValue;
}

export interface ArenaAdapterServer {
  url: string;
  close: () => Promise<void>;
}

export interface CreateArenaAdapterOptions {
  host: string;
  port: number;
  profile: ArenaAdapterProfile;
  chat?: (request: ArenaAdapterChatRequest) => Promise<ArenaAdapterChatResult>;
  workflow?: (request: ArenaAdapterWorkflowRequest) => Promise<ArenaAdapterWorkflowResult>;
}

type ArenaAdapterHandlers = Pick<CreateArenaAdapterOptions, 'chat' | 'workflow'>;

const DEFAULT_OPENAI_MODEL_ID = 'featherlane-ai-target';

export async function createArenaAdapter({
  host,
  port,
  profile,
  chat,
  workflow,
}: CreateArenaAdapterOptions): Promise<ArenaAdapterServer> {
  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    if (handleCors(req, res)) return;

    void handleRequest(req, res, profile, { chat, workflow }).catch((error) => {
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
  handlers: ArenaAdapterHandlers,
): Promise<void> {
  const path = requestPath(req);

  if (req.method === 'GET' && path === '/health') {
    writeJson(res, 200, { ok: true });
    return;
  }

  if (req.method === 'GET' && path === '/arena/profile') {
    writeJson(res, 200, profile);
    return;
  }

  if (req.method === 'GET' && path === '/v1/models') {
    writeJson(res, 200, openAiModelList(profile));
    return;
  }

  if (req.method === 'GET' && path.startsWith('/v1/models/')) {
    writeJson(res, 200, openAiModel(profile, path.slice('/v1/models/'.length)));
    return;
  }

  if (req.method === 'POST' && path === '/v1/chat/completions') {
    await handleOpenAiChat(req, res, profile, handlers.chat);
    return;
  }

  if (req.method === 'POST' && path === '/arena/chat') {
    await handleArenaChat(req, res, profile, handlers.chat);
    return;
  }

  if (req.method === 'POST' && path === '/arena/workflow') {
    if (handlers.workflow === undefined) {
      writeJson(res, 404, { error: 'workflow surface is not available for this adapter' });
      return;
    }

    const body = await readJsonRequest(req);
    const request = parseWorkflowRequest(body);
    if (request === null) {
      writeJson(res, 400, {
        error:
          'expected JSON body with a non-empty documentName and either documentText or documentBase64',
      });
      return;
    }

    const startedAt = Date.now();
    const result = await handlers.workflow(request);
    writeJson(res, 200, {
      agent: profile.displayName,
      ...result,
      latencyMs: Date.now() - startedAt,
    });
    return;
  }

  writeJson(res, 404, { error: 'not found' });
}

function requestPath(req: IncomingMessage): string {
  return new URL(req.url ?? '/', 'http://127.0.0.1').pathname;
}

async function handleOpenAiChat(
  req: IncomingMessage,
  res: ServerResponse,
  profile: ArenaAdapterProfile,
  chat: CreateArenaAdapterOptions['chat'],
): Promise<void> {
  if (chat === undefined) {
    writeJson(res, 404, openAiError('chat surface is not available for this adapter'));
    return;
  }

  const request = withHeaderSession(parseOpenAiChatRequest(await readJsonRequest(req)), req);
  if (request === null) {
    writeJson(res, 400, openAiError('expected OpenAI chat completion body with a user message'));
    return;
  }

  const startedAt = Date.now();
  const result = await chat(request);
  writeJson(res, 200, openAiChatCompletion(profile, result, startedAt));
}

async function handleArenaChat(
  req: IncomingMessage,
  res: ServerResponse,
  profile: ArenaAdapterProfile,
  chat: CreateArenaAdapterOptions['chat'],
): Promise<void> {
  if (chat === undefined) {
    writeJson(res, 404, { error: 'chat surface is not available for this adapter' });
    return;
  }

  const request = withHeaderSession(parseChatRequest(await readJsonRequest(req)), req);
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
}

function openAiModelList(profile: ArenaAdapterProfile): object {
  return {
    object: 'list',
    data: [openAiModel(profile)],
  };
}

function openAiModel(profile: ArenaAdapterProfile, requestedModel = ''): object {
  return {
    id: decodeURIComponent(requestedModel) || profile.model || DEFAULT_OPENAI_MODEL_ID,
    object: 'model',
    created: 0,
    owned_by: 'featherlane-ai-demo',
  };
}

function openAiChatCompletion(
  profile: ArenaAdapterProfile,
  result: ArenaAdapterChatResult,
  startedAt: number,
): object {
  return {
    id: `chatcmpl-${Date.now()}`,
    object: 'chat.completion',
    created: Math.floor(startedAt / 1000),
    model: profile.model ?? DEFAULT_OPENAI_MODEL_ID,
    choices: [
      {
        index: 0,
        message: {
          role: 'assistant',
          content: result.content,
        },
        finish_reason: result.finishReason,
      },
    ],
    usage: {
      prompt_tokens: 0,
      completion_tokens: 0,
      total_tokens: 0,
    },
    'featherlane-ai': {
      agent: profile.displayName,
      effect: result.effect,
      phase: result.phase,
      traceId: result.traceId,
      latencyMs: Date.now() - startedAt,
    },
  };
}

function openAiError(message: string): object {
  return { error: { message } };
}

function parseChatRequest(body: ArenaJsonValue): ArenaAdapterChatRequest | null {
  if (!isJsonObject(body) || typeof body.message !== 'string' || body.message.trim() === '') {
    return null;
  }

  return {
    message: body.message,
    ...(typeof body.sessionId === 'string' && body.sessionId.trim() !== ''
      ? { sessionId: body.sessionId.trim() }
      : {}),
  };
}

function parseOpenAiChatRequest(body: ArenaJsonValue): ArenaAdapterChatRequest | null {
  if (!isJsonObject(body) || !Array.isArray(body.messages)) return null;
  const sessionId = openAiSessionId(body);

  for (let index = body.messages.length - 1; index >= 0; index -= 1) {
    const message = body.messages[index];
    if (!isJsonObject(message) || message.role !== 'user') continue;
    const content = openAiMessageContentToText(message.content);
    if (content.trim() !== '') return { message: content, ...(sessionId ? { sessionId } : {}) };
  }

  return null;
}

function withHeaderSession(
  request: ArenaAdapterChatRequest | null,
  req: IncomingMessage,
): ArenaAdapterChatRequest | null {
  if (request === null || request.sessionId !== undefined) return request;
  const sessionId = headerValue(req.headers['x-featherlane-ai-session-id']);
  return sessionId === undefined ? request : { ...request, sessionId };
}

function headerValue(value: string | string[] | undefined): string | undefined {
  const raw = Array.isArray(value) ? value[0] : value;
  const trimmed = raw?.trim();
  return trimmed ? trimmed : undefined;
}

function openAiSessionId(body: Record<string, ArenaJsonValue>): string | undefined {
  const metadata = body.metadata;
  if (!isJsonObject(metadata)) return undefined;
  for (const key of ['featherlane_ai_session_id', 'sessionId']) {
    const value = metadata[key];
    if (typeof value === 'string' && value.trim() !== '') return value.trim();
  }
  return undefined;
}

function openAiMessageContentToText(content: ArenaJsonValue | undefined): string {
  if (typeof content === 'string') return content;
  if (!Array.isArray(content)) return '';

  const parts: string[] = [];
  for (const part of content) {
    if (!isJsonObject(part)) continue;
    if (typeof part.text === 'string') {
      parts.push(part.text);
    }
  }
  return parts.join('\n');
}

function parseWorkflowRequest(body: ArenaJsonValue): ArenaAdapterWorkflowRequest | null {
  if (
    !isJsonObject(body) ||
    typeof body.documentName !== 'string' ||
    body.documentName.trim() === ''
  ) {
    return null;
  }

  const documentText =
    typeof body.documentText === 'string' && body.documentText.trim() !== ''
      ? body.documentText
      : undefined;
  const documentBase64 =
    typeof body.documentBase64 === 'string' && body.documentBase64.trim() !== ''
      ? body.documentBase64
      : undefined;
  if (documentText === undefined && documentBase64 === undefined) return null;

  return {
    documentName: body.documentName,
    ...(documentText !== undefined ? { documentText } : {}),
    ...(documentBase64 !== undefined ? { documentBase64 } : {}),
    ...(typeof body.documentMimeType === 'string' && body.documentMimeType.trim() !== ''
      ? { documentMimeType: body.documentMimeType }
      : {}),
    ...(typeof body.workflowGoal === 'string' && body.workflowGoal.trim() !== ''
      ? { workflowGoal: body.workflowGoal }
      : {}),
  };
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
