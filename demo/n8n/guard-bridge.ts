import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';

import { guard, type Channel } from '@trustloopguard/sdk';

import { DEFAULT_AGENT_ID, createClient, registerDemoProfile } from '../shared/env';
import { blockedReply, escalatedReply } from '../shared/replies';

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };
type JsonRecord = { [key: string]: JsonValue };

interface GuardBridgeRequest {
  input: string;
  draft: string;
  agentId: string;
  channel: Channel;
}

const PORT = Number.parseInt(process.env.TL_N8N_BRIDGE_PORT ?? '8787', 10);

async function main(): Promise<void> {
  try {
    await registerDemoProfile(DEFAULT_AGENT_ID);
  } catch (error) {
    process.stderr.write(
      `could not register profile, continuing anyway: ${
        error instanceof Error ? error.message : String(error)
      }\n\n`,
    );
  }

  const client = createClient();
  const server = createServer(async (req, res) => {
    try {
      if (req.method === 'GET' && req.url === '/health') {
        sendJson(res, 200, { ok: true });
        return;
      }

      if (req.method !== 'POST' || req.url !== '/guard') {
        sendJson(res, 404, { error: 'not_found' });
        return;
      }

      const payload = normalizeRequest(await readJson(req));
      let guardEvent = {
        trace_id: '',
        verdict: 'allow',
        branch: 'allow',
        latency_ms: 0,
      };

      const guardedOutput = await guard({
        client,
        agentId: payload.agentId,
        channel: payload.channel,
        input: payload.input,
        draft: payload.draft,
        context: {
          demo_surface: 'n8n',
          n8n_execution_id: headerValue(req, 'x-n8n-execution-id') ?? 'manual-demo',
        },
        onBlock: blockedReply,
        onEscalate: escalatedReply,
        log: (event) => {
          guardEvent = event;
        },
      });

      sendJson(res, 200, {
        guardedOutput,
        originalDraft: payload.draft,
        guard: {
          verdict: guardEvent.verdict,
          branch: guardEvent.branch,
          traceId: guardEvent.trace_id,
          latencyMs: guardEvent.latency_ms,
        },
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      sendJson(res, 400, { error: 'bad_request', message });
    }
  });

  server.listen(PORT, '127.0.0.1', () => {
    process.stdout.write(`n8n guard bridge listening on http://127.0.0.1:${PORT}/guard\n`);
  });
}

function normalizeRequest(body: JsonRecord): GuardBridgeRequest {
  const input = valueAsString(body.input, 'input');
  const draft = valueAsString(body.draft, 'draft');
  const agentId = typeof body.agentId === 'string' ? body.agentId : DEFAULT_AGENT_ID;
  const channel = normalizeChannel(body.channel);
  return { input, draft, agentId, channel };
}

function normalizeChannel(value: JsonValue | undefined): Channel {
  if (value === 'voice' || value === 'chat' || value === 'email') return value;
  return 'chat';
}

function valueAsString(value: JsonValue | undefined, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`"${field}" must be a non-empty string`);
  }
  return value;
}

async function readJson(req: IncomingMessage): Promise<JsonRecord> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  const raw = Buffer.concat(chunks).toString('utf-8');
  const parsed = JSON.parse(raw || '{}') as JsonValue;
  if (!isJsonRecord(parsed)) throw new Error('request body must be a JSON object');
  return parsed;
}

function isJsonRecord(value: JsonValue): value is JsonRecord {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function headerValue(req: IncomingMessage, name: string): string | null {
  const value = req.headers[name];
  if (typeof value === 'string') return value;
  if (Array.isArray(value)) return value[0] ?? null;
  return null;
}

function sendJson(res: ServerResponse, status: number, body: JsonRecord): void {
  res.writeHead(status, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
}

main().catch((error) => {
  process.stderr.write(
    `n8n guard bridge failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exit(2);
});
