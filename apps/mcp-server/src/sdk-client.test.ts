import { describe, expect, it } from 'vitest';

import { Client } from '@trustloopguard/sdk';

describe('Client MCP support methods', () => {
  it('lists traces with optional filters', async () => {
    const requests: string[] = [];
    const client = new Client({
      baseUrl: 'http://server.test',
      fetchImpl: async (input) => {
        requests.push(typeof input === 'string' ? input : input instanceof URL ? input.href : input.url);
        return new Response(JSON.stringify({ traces: [] }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      },
    });

    await client.listTraces({ limit: 10, sessionId: 'sess_1' });

    expect(requests).toEqual(['http://server.test/v1/traces?limit=10&session_id=sess_1']);
  });

  it('manages tool metadata through the Rust API', async () => {
    const requests: Array<{ url: string; init: RequestInit }> = [];
    const client = new Client({
      baseUrl: 'http://server.test',
      fetchImpl: async (input, init) => {
        requests.push({
          url: typeof input === 'string' ? input : input instanceof URL ? input.href : input.url,
          init: init ?? {},
        });
        return new Response(JSON.stringify({ tools: [] }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      },
    });

    await client.listToolMetadata();
    await client.upsertToolMetadata({
      tool: 'send_email',
      side_effect: 'external_communication',
      reversible: false,
      params: [],
      enabled: true,
    });

    expect(requests[0]!.url).toBe('http://server.test/v1/tool-metadata');
    expect(requests[0]!.init.method).toBe('GET');
    expect(requests[1]!.url).toBe('http://server.test/v1/tool-metadata');
    expect(requests[1]!.init.method).toBe('POST');
  });
});
