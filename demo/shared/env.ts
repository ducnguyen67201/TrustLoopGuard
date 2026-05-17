import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Client } from '@trustloopguard/sdk';
import { parseDocument } from 'yaml';

export const SERVER_URL = process.env.TL_SERVER_URL ?? 'http://127.0.0.1:8080';
export const API_KEY = process.env.TL_API_KEY;
export const DEFAULT_AGENT_ID = process.env.TL_AGENT_ID ?? 'demo-acme-support';

export function createClient(): Client {
  return new Client({ baseUrl: SERVER_URL, apiKey: API_KEY });
}

export function demoRoot(): string {
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
