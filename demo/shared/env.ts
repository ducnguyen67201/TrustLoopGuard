import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Client } from '@trustloopguard/sdk';
import { parseDocument } from 'yaml';

import { loadDemoEnvForCurrentScript } from './load-env';

loadDemoEnvForCurrentScript();

export const SERVER_URL = process.env.TL_SERVER_URL ?? 'http://127.0.0.1:8080';
export const API_KEY = process.env.TL_API_KEY;
export const DEFAULT_AGENT_ID = process.env.TL_AGENT_ID ?? 'demo-acme-support';
export const WORKSPACE_ID = process.env.TL_WORKSPACE_ID;
export const ADMIN_USER_ID = cleanOptionalEnv(process.env.TL_ADMIN_USER_ID);
export const HEALTHCARE_DEMO_API_KEY = cleanOptionalEnv(
  process.env.TL_HEALTHCARE_DEMO_API_KEY,
);
export const REFUND_GRANT_ID = cleanOptionalEnv(process.env.TL_REFUND_GRANT_ID);
export const OPENAI_API_KEY = process.env.OPENAI_API_KEY;
export const OPENAI_MODEL = process.env.OPENAI_MODEL ?? 'gpt-4.1-mini';
// The customer's workflow splits document understanding across two models
// (classification then schema-based extraction). Mirror that shape; both
// default to OPENAI_MODEL so nothing breaks without the extra envs.
export const OPENAI_CLASSIFY_MODEL = process.env.OPENAI_CLASSIFY_MODEL ?? OPENAI_MODEL;
export const OPENAI_EXTRACT_MODEL = process.env.OPENAI_EXTRACT_MODEL ?? OPENAI_MODEL;
// Owned "world" sink the demo agent actually POSTs side effects to (loopback only).
// The bundled runner starts it on this URL; agents read it to know where to act.
export const AGENT_DEMO_WORLD_HOST = process.env.AGENT_DEMO_WORLD_HOST ?? '127.0.0.1';
export const AGENT_DEMO_WORLD_PORT = parsePort(process.env.AGENT_DEMO_WORLD_PORT, 9120);
export const AGENT_DEMO_SINK_URL =
  process.env.AGENT_DEMO_SINK_URL ?? `http://${AGENT_DEMO_WORLD_HOST}:${AGENT_DEMO_WORLD_PORT}`;

/** A malformed port env must not silently produce `http://host:NaN`. */
function parsePort(raw: string | undefined, fallback: number): number {
  if (raw === undefined || raw.trim() === '') return fallback;
  const port = Number.parseInt(raw, 10);
  return Number.isInteger(port) && port > 0 && port <= 65535 ? port : fallback;
}

export function createClient(): Client {
  const fetchImpl = fetchWithTrustLoopContext();
  return new Client({
    baseUrl: SERVER_URL,
    apiKey: API_KEY,
    ...(fetchImpl === undefined ? {} : { fetchImpl }),
  });
}

function cleanOptionalEnv(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed === undefined || trimmed === '' ? undefined : trimmed;
}

function demoRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), '..');
}

export async function registerDemoProfile(agentId = DEFAULT_AGENT_ID): Promise<void> {
  const yamlPath = resolve(demoRoot(), 'agents', 'acme-support-v3.yaml');
  const profile = parseDocument(readFileSync(yamlPath, 'utf-8'));
  profile.set('agent_id', agentId);

  const headers: Record<string, string> = {
    'content-type': 'application/yaml',
  };
  if (API_KEY) headers.authorization = `Bearer ${API_KEY}`;
  if (WORKSPACE_ID) headers['x-tlg-workspace-id'] = WORKSPACE_ID;
  if (ADMIN_USER_ID) headers['x-tlg-user-id'] = ADMIN_USER_ID;

  const res = await fetch(`${SERVER_URL}/v1/agents`, {
    method: 'POST',
    headers,
    body: String(profile),
  });
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new Error(`register failed: ${res.status} ${body}`);
  }

  process.stdout.write(`registered agent profile "${agentId}"\n\n`);
}

function fetchWithTrustLoopContext(): typeof fetch | undefined {
  const workspaceId = cleanOptionalEnv(WORKSPACE_ID);
  const adminUserId = ADMIN_USER_ID;
  if (workspaceId === undefined && adminUserId === undefined) return undefined;
  return ((input: RequestInfo | URL, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    if (workspaceId !== undefined) headers.set('x-tlg-workspace-id', workspaceId);
    if (adminUserId !== undefined) headers.set('x-tlg-user-id', adminUserId);
    return fetch(input, { ...init, headers });
  }) as typeof fetch;
}
