import { describe, expect, it, vi } from 'vitest';

import { Client } from '../src';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

describe('Client policy authoring methods', () => {
  it('binds the default global fetch for browser runtimes', async () => {
    const originalFetch = globalThis.fetch;
    const fetchSpy = vi.fn(async function (this: typeof globalThis) {
      expect(this).toBe(globalThis);
      return jsonResponse({ policies: [] });
    }) as unknown as typeof fetch;
    globalThis.fetch = fetchSpy;

    try {
      const client = new Client({ baseUrl: 'http://server.test' });
      await client.listPolicies();
    } finally {
      globalThis.fetch = originalFetch;
    }
  });

  it('lists policies from /v1/policies', async () => {
    const fetchSpy = vi.fn(async () =>
      jsonResponse({
        policies: [
          {
            id: 'refund-guarantee',
            description: 'Prevent refund promises',
            severity: 'high',
            enabled: true,
          },
        ],
      }),
    ) as unknown as typeof fetch;
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const policies = await client.listPolicies();

    expect(policies.policies[0]!.id).toBe('refund-guarantee');
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/policies');
    expect((init as RequestInit).method).toBe('GET');
  });

  it('publishes policy YAML with the YAML content type', async () => {
    const fetchSpy = vi.fn(async () =>
      jsonResponse(
        {
          id: 'refund-guarantee',
          description: 'Prevent refund promises',
          severity: 'high',
          enabled: true,
          source_yaml: 'id: refund-guarantee',
        },
        201,
      ),
    ) as unknown as typeof fetch;
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const policy = await client.upsertPolicy('id: refund-guarantee');

    expect(policy.id).toBe('refund-guarantee');
    const [, init] = fetchSpy.mock.calls[0]!;
    expect((init as RequestInit).method).toBe('POST');
    expect(((init as RequestInit).headers as Record<string, string>)['content-type']).toBe(
      'application/yaml',
    );
    expect((init as RequestInit).body).toBe('id: refund-guarantee');
  });

  it('updates enabled state through the dedicated endpoint', async () => {
    const fetchSpy = vi.fn(async () =>
      jsonResponse({
        id: 'refund-guarantee',
        description: 'Prevent refund promises',
        severity: 'high',
        enabled: false,
        source_yaml: 'id: refund-guarantee',
      }),
    ) as unknown as typeof fetch;
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const policy = await client.setPolicyEnabled('refund-guarantee', false);

    expect(policy.enabled).toBe(false);
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/policies/refund-guarantee/enabled');
    expect((init as RequestInit).method).toBe('PATCH');
    expect(JSON.parse((init as RequestInit).body as string)).toEqual({ enabled: false });
  });

  it('validates policy source without saving it', async () => {
    const fetchSpy = vi.fn(async () =>
      jsonResponse({
        valid: false,
        errors: [{ path: 'id', message: 'id is required' }],
      }),
    ) as unknown as typeof fetch;
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const result = await client.validatePolicy('action: block');

    expect(result.valid).toBe(false);
    expect(result.errors[0]!.path).toBe('id');
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/policies/validate');
    expect((init as RequestInit).method).toBe('POST');
  });
});
