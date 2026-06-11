import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { API_KEY, SERVER_URL, WORKSPACE_ID, demoRoot } from '../shared/env';

const POLICY_PATHS = [
  resolve(demoRoot(), 'showdown', 'policies', 'block-data-exfiltration.yaml'),
  resolve(demoRoot(), '..', 'policies', 'refund-promise.yaml'),
];

export async function seedShowdownPolicies(): Promise<void> {
  for (const path of POLICY_PATHS) {
    await upsertPolicy(path);
  }
}

async function upsertPolicy(yamlPath: string): Promise<void> {
  const headers: Record<string, string> = {
    'content-type': 'application/yaml',
  };
  if (API_KEY) headers.authorization = `Bearer ${API_KEY}`;
  if (WORKSPACE_ID) headers['x-tlg-workspace-id'] = WORKSPACE_ID;

  const res = await fetch(`${SERVER_URL}/v1/policies`, {
    method: 'POST',
    headers,
    body: readFileSync(yamlPath, 'utf-8'),
  });
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new Error(`policy upsert failed for ${yamlPath}: ${res.status} ${body}`);
  }
}
