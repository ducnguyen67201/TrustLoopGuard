import { describe, expect, it } from 'vitest';

import { Client, NotFound, RetryConfig, Unavailable, Unprocessable } from '../src';
import { jsonResponse, mockFetch } from './test-utils';

function oneShotRetry(): RetryConfig {
  return { maxAttempts: 1, totalBudgetS: 0.05, baseDelayS: 0.001, maxDelayS: 0.002 };
}

const GENERATE_BODY = {
  generated: [
    {
      id: 'no-pii-leak',
      description: 'Block emails.',
      severity: 'high',
      enabled: false,
      source_yaml: 'id: no-pii-leak\n',
    },
    {
      id: 'no-medical-claims',
      description: 'Block medical claims.',
      severity: 'high',
      enabled: false,
      source_yaml: 'id: no-medical-claims\n',
    },
  ],
};

describe('Client guardrail methods', () => {
  it('generateGuardrails POSTs to the encoded path and returns disabled policies', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse(GENERATE_BODY),
    );
    const client = new Client({
      baseUrl: 'http://server.test',
      apiKey: 'secret',
      fetchImpl: fetchSpy,
    });

    const out = await client.generateGuardrails('baker-9000');
    expect(out.generated).toHaveLength(2);
    expect(out.generated.every((p) => !p.enabled)).toBe(true);

    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/agents/baker-9000/guardrails/generate');
    expect((init as RequestInit).method).toBe('POST');
    const headers = (init as RequestInit).headers as Record<string, string>;
    expect(headers['authorization']).toBe('Bearer secret');
  });

  it('encodeURIComponent escapes slashes and spaces in the agent id', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse(GENERATE_BODY),
    );
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    await client.generateGuardrails('team/baker one');

    const [url] = fetchSpy.mock.calls[0]!;
    expect(url).toBe(
      'http://server.test/v1/agents/team%2Fbaker%20one/guardrails/generate',
    );
  });

  it('maps 404 to NotFound', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse(
        {
          code: 'not_found',
          message: 'agent ghost not found',
          retriable: false,
          details: null,
        },
        404,
      ),
    );
    const client = new Client({
      baseUrl: 'http://server.test',
      fetchImpl: fetchSpy,
      retry: oneShotRetry(),
    });

    await expect(client.generateGuardrails('ghost')).rejects.toBeInstanceOf(NotFound);
  });

  it('maps 422 to Unprocessable', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse(
        {
          code: 'unprocessable',
          message: 'no system_prompt',
          retriable: false,
          details: null,
        },
        422,
      ),
    );
    const client = new Client({
      baseUrl: 'http://server.test',
      fetchImpl: fetchSpy,
      retry: oneShotRetry(),
    });

    await expect(client.generateGuardrails('silent')).rejects.toBeInstanceOf(
      Unprocessable,
    );
  });

  it('maps 503 to Unavailable', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse(
        {
          code: 'unavailable',
          message: 'no LLM',
          retriable: true,
          details: null,
        },
        503,
      ),
    );
    const client = new Client({
      baseUrl: 'http://server.test',
      fetchImpl: fetchSpy,
      retry: oneShotRetry(),
    });

    await expect(client.generateGuardrails('a')).rejects.toBeInstanceOf(Unavailable);
  });

  it('listGuardrails GETs the agent path and returns owned policies', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse({
        policies: [
          {
            id: 'no-pii-leak',
            description: 'Block emails.',
            severity: 'high',
            enabled: false,
          },
        ],
      }),
    );
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const out = await client.listGuardrails('baker-9000');
    expect(out.policies).toHaveLength(1);
    expect(out.policies[0]!.id).toBe('no-pii-leak');

    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/agents/baker-9000/guardrails');
    expect((init as RequestInit).method).toBe('GET');
  });

  it('listGuardrails returns empty for unknown agents', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse({ policies: [] }),
    );
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const out = await client.listGuardrails('ghost');
    expect(out.policies).toEqual([]);
  });
});
