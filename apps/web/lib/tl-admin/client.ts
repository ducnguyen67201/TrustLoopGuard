import 'server-only';

import { z } from 'zod';

import { env } from '@/env';

const apiKeyViewSchema = z.object({
  id: z.string(),
  user_id: z.string(),
  name: z.string(),
  prefix: z.string(),
  last_used_at: z.string().datetime().nullable(),
  created_at: z.string().datetime(),
  revoked_at: z.string().datetime().nullable(),
});

const listResponseSchema = z.object({
  keys: z.array(apiKeyViewSchema),
});

const createResponseSchema = z.object({
  id: z.string(),
  plaintext: z.string(),
  prefix: z.string(),
  name: z.string(),
  user_id: z.string(),
  created_at: z.string().datetime(),
});

export type ApiKeyView = z.infer<typeof apiKeyViewSchema>;
export type CreatedApiKey = z.infer<typeof createResponseSchema>;

export class AdminApiError extends Error {
  constructor(message: string, public readonly status: number) {
    super(message);
    this.name = 'AdminApiError';
  }
}

function requireAdminKey(): string {
  if (!env.TL_ADMIN_KEY) {
    throw new AdminApiError(
      'API key management is not configured on this deployment. Set TL_ADMIN_KEY to enable it.',
      503,
    );
  }
  return env.TL_ADMIN_KEY;
}

function headers(): HeadersInit {
  return {
    Authorization: `Bearer ${requireAdminKey()}`,
    'Content-Type': 'application/json',
  };
}

function url(path: string): string {
  return new URL(path, env.TL_SERVER_INTERNAL_URL).toString();
}

export async function listKeys(userId: string): Promise<ApiKeyView[]> {
  const res = await fetch(
    url(`/v1/admin/keys?user_id=${encodeURIComponent(userId)}`),
    { headers: headers(), cache: 'no-store' },
  );
  if (!res.ok) {
    throw new AdminApiError(`listKeys failed (${res.status})`, res.status);
  }
  const parsed = listResponseSchema.parse(await res.json());
  return parsed.keys;
}

export async function createKey(
  userId: string,
  name: string,
): Promise<CreatedApiKey> {
  const res = await fetch(url('/v1/admin/keys'), {
    method: 'POST',
    headers: headers(),
    body: JSON.stringify({ user_id: userId, name }),
    cache: 'no-store',
  });
  if (!res.ok) {
    throw new AdminApiError(`createKey failed (${res.status})`, res.status);
  }
  return createResponseSchema.parse(await res.json());
}

export async function revokeKey(userId: string, keyId: string): Promise<void> {
  const res = await fetch(
    url(`/v1/admin/keys/${encodeURIComponent(keyId)}?user_id=${encodeURIComponent(userId)}`),
    { method: 'DELETE', headers: headers(), cache: 'no-store' },
  );
  if (!res.ok && res.status !== 404) {
    throw new AdminApiError(`revokeKey failed (${res.status})`, res.status);
  }
}
