// `submitEvent()` tests. Builds a `Client` with a mock `fetchImpl` so no
// network is involved.

import { describe, expect, it } from 'vitest';

import { Client, Internal, guard, type Decision, type GuardEvent } from '../src';
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

function runSummary(id = '018f1111-1111-7111-8111-111111111111'): Record<string, unknown> {
  return {
    id,
    workspace_id: 'ws_1',
    environment_id: 'production',
    environment: 'production',
    agent_id: 'agent-1',
    kind: 'chat_session',
    status: 'running',
    external_id: null,
    metadata: {},
    started_at: '2026-01-01T00:00:00Z',
    ended_at: null,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    trace_count: 0,
    blocked_count: 0,
    rewritten_count: 0,
    escalated_count: 0,
    p95_latency_ms: null,
  };
}

function runEventSummary(): Record<string, unknown> {
  return {
    id: '018f2222-2222-7222-8222-222222222222',
    workspace_id: 'ws_1',
    run_id: '018f1111-1111-7111-8111-111111111111',
    sequence: 1,
    kind: 'user_turn',
    label: null,
    input_summary: null,
    output_summary: null,
    metadata: {},
    occurred_at: '2026-01-01T00:00:00Z',
    created_at: '2026-01-01T00:00:00Z',
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
    const client = new Client({ baseUrl: 'http://x', fetchImpl });

    await expect(client.submitEvent(sendEmailEvent())).rejects.toBeInstanceOf(Internal);
  });

  it('inherits active run and run event context', async () => {
    const bodies: unknown[] = [];
    const fetchImpl = mockFetch(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      if (url === 'http://x/v1/runs') {
        return new Response(JSON.stringify(runSummary()), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      if (url === 'http://x/v1/runs/018f1111-1111-7111-8111-111111111111/events') {
        return new Response(JSON.stringify(runEventSummary()), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      if (url === 'http://x/v1/runs/018f1111-1111-7111-8111-111111111111') {
        return new Response(JSON.stringify({}), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      bodies.push(JSON.parse(String(init?.body)));
      return new Response(JSON.stringify(observeOnlyDecision()), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    });
    const client = new Client({
      baseUrl: 'http://x',
      fetchImpl,
      retry: { maxAttempts: 1, baseDelayS: 0, maxDelayS: 0, totalTimeoutS: 1 },
    });

    await client.withRun({ agentId: 'agent-1', kind: 'chat_session' }, async (run) => {
      await client.submitEvent(sendEmailEvent());
      await run.withEvent({ kind: 'user_turn', metadata: {} }, async () => {
        await guard({
          client,
          agentId: 'agent-1',
          input: 'refund order 1',
          draft: 'I can help.',
          onBlock: () => 'blocked',
          onEscalate: () => 'escalated',
        });
      });
    });

    expect((bodies[0] as GuardEvent).principal.run_id).toBe(
      '018f1111-1111-7111-8111-111111111111',
    );
    expect((bodies[0] as GuardEvent).principal.run_event_id).toBeUndefined();
    expect((bodies[1] as GuardEvent).principal.run_id).toBe(
      '018f1111-1111-7111-8111-111111111111',
    );
    expect((bodies[1] as GuardEvent).principal.run_event_id).toBe(
      '018f2222-2222-7222-8222-222222222222',
    );
  });

  it('preserves explicit run fields and builds tool-call events', async () => {
    const bodies: GuardEvent[] = [];
    const fetchImpl = mockFetch(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      if (url === 'http://x/v1/runs') {
        return new Response(JSON.stringify(runSummary()), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      if (url === 'http://x/v1/runs/018f1111-1111-7111-8111-111111111111') {
        return new Response(JSON.stringify({}), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      if (url === 'http://x/v1/runs/018f1111-1111-7111-8111-111111111111/events') {
        return new Response(JSON.stringify(runEventSummary()), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      bodies.push(JSON.parse(String(init?.body)) as GuardEvent);
      return new Response(JSON.stringify(observeOnlyDecision()), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    });
    const client = new Client({
      baseUrl: 'http://x',
      fetchImpl,
      retry: { maxAttempts: 1, baseDelayS: 0, maxDelayS: 0, totalTimeoutS: 1 },
    });
    const explicit = sendEmailEvent();
    explicit.principal.run_id = 'explicit-run';

    await client.withRun({ agentId: 'agent-1', kind: 'chat_session' }, async (run) => {
      await run.withEvent({ kind: 'user_turn', metadata: {} }, async () => {
        await client.submitEvent(explicit);
      });
      await client.guardToolCall({
        agentId: 'agent-1',
        operation: 'issue_refund',
        parameters: { orderId: 'o_1' },
        sideEffect: 'api_mutation',
        sources: [{ id: 'input', origin: 'user', labels: {} }],
        provenance: { orderId: ['input'] },
      });
    });

    expect(bodies[0].principal.run_id).toBe('explicit-run');
    expect(bodies[0].principal.run_event_id).toBeUndefined();
    expect(bodies[1].kind).toBe('tool.call.proposed');
    expect(bodies[1].action.operation).toBe('issue_refund');
    expect(bodies[1].principal.run_id).toBe('018f1111-1111-7111-8111-111111111111');
    expect(bodies[1].principal.run_event_id).toBeUndefined();
  });

  it('keeps the body error when failed-run cleanup also fails', async () => {
    const fetchImpl = mockFetch(async (input) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      if (url === 'http://x/v1/runs') {
        return new Response(JSON.stringify(runSummary()), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      throw new Error('finish failed');
    });
    const client = new Client({
      baseUrl: 'http://x',
      fetchImpl,
      retry: { maxAttempts: 1, baseDelayS: 0, maxDelayS: 0, totalTimeoutS: 1 },
    });

    await expect(
      client.withRun({ agentId: 'agent-1' }, async () => {
        throw new Error('body failed');
      }),
    ).rejects.toThrow('body failed');
  });
});
