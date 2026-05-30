import { describe, expect, it } from 'vitest';

import { Client } from '../src';
import { jsonResponse, mockFetch } from './test-utils';

const RUN_BODY = {
  id: '018f1111-1111-7111-8111-111111111111',
  workspace_id: 'ws_test',
  environment_id: 'production',
  environment: 'production',
  agent_id: 'support-agent',
  kind: 'chat_session',
  status: 'running',
  external_id: 'chat-123',
  metadata: {},
  started_at: '2026-05-17T00:00:00Z',
  ended_at: null,
  created_at: '2026-05-17T00:00:00Z',
  updated_at: '2026-05-17T00:00:00Z',
  trace_count: 0,
  blocked_count: 0,
  rewritten_count: 0,
  escalated_count: 0,
  p95_latency_ms: null,
};

const RUN_EVENT_BODY = {
  id: '018f2222-2222-7222-8222-222222222222',
  workspace_id: 'ws_test',
  run_id: RUN_BODY.id,
  sequence: 1,
  kind: 'user_turn',
  label: 'Turn 1',
  input_summary: 'Customer asks about a refund',
  output_summary: null,
  metadata: {},
  occurred_at: '2026-05-17T00:00:01Z',
  created_at: '2026-05-17T00:00:01Z',
};

describe('Client run methods', () => {
  it('startRun POSTs run metadata and returns the run summary', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(RUN_BODY, 201));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const run = await client.startRun({
      agent_id: 'support-agent',
      kind: 'chat_session',
      external_id: 'chat-123',
    });

    expect(run.id).toBe(RUN_BODY.id);
    const traceCount: number = run.trace_count;
    expect(traceCount).toBe(0);
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/runs');
    expect((init as RequestInit).method).toBe('POST');
    expect(JSON.parse(String((init as RequestInit).body))).toEqual({
      agent_id: 'support-agent',
      kind: 'chat_session',
      external_id: 'chat-123',
      metadata: {},
    });
  });

  it('finishRun PATCHes completed status', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse({ ...RUN_BODY, status: 'completed', ended_at: '2026-05-17T00:01:00Z' }),
    );
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const run = await client.finishRun(RUN_BODY.id);

    expect(run.status).toBe('completed');
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe(`http://server.test/v1/runs/${RUN_BODY.id}`);
    expect((init as RequestInit).method).toBe('PATCH');
    expect(JSON.parse(String((init as RequestInit).body))).toEqual({ status: 'completed' });
  });

  it('createRunEvent POSTs timeline context', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(RUN_EVENT_BODY, 201));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const event = await client.createRunEvent(RUN_BODY.id, {
      kind: 'user_turn',
      label: 'Turn 1',
      input_summary: 'Customer asks about a refund',
    });

    expect(event.id).toBe(RUN_EVENT_BODY.id);
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe(`http://server.test/v1/runs/${RUN_BODY.id}/events`);
    expect((init as RequestInit).method).toBe('POST');
    expect(JSON.parse(String((init as RequestInit).body))).toEqual({
      kind: 'user_turn',
      label: 'Turn 1',
      input_summary: 'Customer asks about a refund',
      metadata: {},
    });
  });
});
