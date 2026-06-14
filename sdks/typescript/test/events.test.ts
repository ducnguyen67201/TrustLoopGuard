// `submitEvent()` tests. Builds a `Client` with a mock `fetchImpl` so no
// network is involved.

import { describe, expect, it } from 'vitest';

import { Client, Internal, type Decision, type GuardEvent } from '../src';
import { mockFetch } from './test-utils';

const DEFAULT_EVENT_ALLOW_REASON = 'event allowed: no enforced checker or enabled policy matched';

function sendEmailEvent(): GuardEvent {
  return {
    kind: 'tool.call.proposed',
    principal: {
      workspace_id: 'ws_1',
      environment_id: 'production',
      agent_id: 'agent-1',
    },
    action: {
      operation: 'send_email',
      parameters: { recipient: 'a@b.c', body: 'hi' },
    },
    sources: [
      { id: 'src.user', origin: 'user', labels: {} },
      { id: 'src.web', origin: 'web', labels: {}, kind: 'web_page' },
    ],
    provenance: {
      recipient: ['src.web'],
      body: ['src.user', 'src.web'],
    },
    context: null,
  } as GuardEvent;
}

function observeOnlyDecision(): Record<string, unknown> {
  return {
    trace_id: 't-1',
    verdict: 'allow',
    reason: DEFAULT_EVENT_ALLOW_REASON,
    triggered_policies: [],
    safe_output: null,
    latency_ms: 2,
    tier_results: [],
  };
}

describe('submitEvent', () => {
  it('posts the event to /v1/events with auth and returns the decision', async () => {
    const fetchImpl = mockFetch(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      expect(url).toBe('http://x/v1/events');
      expect(init?.method).toBe('POST');
      const headers = init?.headers as Record<string, string>;
      expect(headers['authorization']).toBe('Bearer secret');
      const body = JSON.parse(String(init?.body)) as GuardEvent;
      expect(body.action.operation).toBe('send_email');
      expect(body.provenance).toEqual({
        recipient: ['src.web'],
        body: ['src.user', 'src.web'],
      });
      return new Response(JSON.stringify(observeOnlyDecision()), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    });
    const client = new Client({ baseUrl: 'http://x', apiKey: 'secret', fetchImpl });

    const decision: Decision = await client.submitEvent(sendEmailEvent());

    expect(decision.verdict).toBe('allow');
    expect(decision.reason).toBe(DEFAULT_EVENT_ALLOW_REASON);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('maps server errors to typed SdkErrors', async () => {
    const fetchImpl = mockFetch(
      async () =>
        new Response(
          JSON.stringify({ code: 'internal', message: 'boom', retriable: false }),
          { status: 500, headers: { 'content-type': 'application/json' } },
        ),
    );
    const client = new Client({
      baseUrl: 'http://x',
      fetchImpl,
      retry: { maxAttempts: 1, baseDelayS: 0, maxDelayS: 0, totalTimeoutS: 1 },
    });

    await expect(client.submitEvent(sendEmailEvent())).rejects.toBeInstanceOf(Internal);
  });
});
