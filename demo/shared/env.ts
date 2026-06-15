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
export const OPENAI_API_KEY = process.env.OPENAI_API_KEY;
export const OPENAI_MODEL = process.env.OPENAI_MODEL ?? 'gpt-4.1-mini';

export function createClient(): Client {
  if (WORKSPACE_ID !== undefined && WORKSPACE_ID.trim() !== '') {
    return new Client({
      baseUrl: SERVER_URL,
      apiKey: API_KEY,
      fetchImpl: fetchWithWorkspace(WORKSPACE_ID.trim()),
    });
  }
  return new Client({ baseUrl: SERVER_URL, apiKey: API_KEY });
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

function fetchWithWorkspace(workspaceId: string): typeof fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    headers.set('x-tlg-workspace-id', workspaceId);
    return fetch(input, { ...init, headers });
  }) as typeof fetch;
}
