// Wraps Client.check() with a zod parse on the response. Keeps every
// caller in apps/web typesafe end-to-end without leaking the SDK's
// (currently incorrect) bigint typing for latency_ms.

import { Client } from '@trustloopguard/sdk';
import type { CheckRequest } from '@trustloopguard/sdk';
import { decisionResponseSchema, type DecisionResponse } from './schemas';
import { getServerUrl } from './server-url';

let cachedClient: Client | null = null;

function getClient(): Client {
  if (cachedClient !== null) return cachedClient;
  cachedClient = new Client({ baseUrl: getServerUrl() });
  return cachedClient;
}

export async function check(req: CheckRequest, signal?: AbortSignal): Promise<DecisionResponse> {
  const raw = await getClient().check(req, signal);
  return decisionResponseSchema.parse(raw);
}
