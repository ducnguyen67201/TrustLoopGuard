import type { ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse } from '@trustloopguard/sdk';
import { z } from 'zod';
import { http } from './http';

const apiKeySchema = z.object({
  id: z.string(),
  name: z.string(),
  prefix: z.string(),
  status: z.string(),
  created_at: z.string(),
  last_used_at: z.string().nullable(),
  created_by: z.string().nullable(),
});

const apiKeyBatchRevokeResponseSchema: z.ZodType<ApiKeyBatchRevokeResponse> = z.object({
  api_keys: z.array(apiKeySchema),
});

export async function revokeApiKeys(
  ids: string[],
  signal?: AbortSignal,
): Promise<ApiKeyBatchRevokeResponse> {
  const body = { ids } satisfies ApiKeyBatchRevokeRequest;
  return http.patch(
    '/api/api-keys/batch/revoke',
    body,
    apiKeyBatchRevokeResponseSchema,
    { signal },
  );
}
