import { describe, expect, it } from 'vitest';

import { Client } from '../src';
import { jsonResponse, mockFetch } from './test-utils';

describe('Client API key admin methods', () => {
  it('revokes multiple API keys through the batch endpoint', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse({
        api_keys: [
          {
            id: 'apk_one',
            name: 'Runtime',
            prefix: 'tl_live_abc',
            status: 'revoked',
            created_at: '2026-01-01T00:00:00Z',
            last_used_at: null,
            created_by: null,
          },
        ],
      }),
    );
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const result = await client.batchRevokeApiKeys(['apk_one']);

    expect(result.api_keys[0]!.status).toBe('revoked');
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/api-keys/batch/revoke');
    expect((init as RequestInit).method).toBe('PATCH');
    expect(JSON.parse((init as RequestInit).body as string)).toEqual({ ids: ['apk_one'] });
  });
});
