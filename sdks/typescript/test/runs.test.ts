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
  agent_id: 'support-agent',
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

  it('finishRun finalizes the completed run', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse({
        run: { ...RUN_BODY, status: 'completed', ended_at: '2026-05-17T00:01:00Z' },
        finalization: {
          finalized_at: '2026-05-17T00:01:00Z',
          boundary_source: 'explicit_sdk',
          boundary_confidence: 'authoritative',
          capture_status: 'waiting',
          capture_deadline: '2026-05-17T00:01:30Z',
          expected_flush_id: null,
        },
        evaluation_status: 'waiting_capture',
      }),
    );
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const run = await client.finishRun(RUN_BODY.id);

    expect(run.status).toBe('completed');
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe(`http://server.test/v1/runs/${RUN_BODY.id}/finalize`);
    expect((init as RequestInit).method).toBe('POST');
    expect(JSON.parse(String((init as RequestInit).body))).toEqual({
      status: 'completed',
      boundary_source: 'explicit_sdk',
    });
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

  it('configures and reads post-run evaluations through typed endpoints', async () => {
    const fetchSpy = mockFetch(async (input, init) => {
      const url = String(input);
      if (url.endsWith('/evaluation-profile')) {
        return jsonResponse({
          workspace_id: 'ws_test',
          environment_id: 'production',
          agent_id: 'support-agent',
          ...JSON.parse(String((init as RequestInit).body)),
          profile_version: 1,
          updated_at: '2026-05-17T00:00:00Z',
        });
      }
      if (url.endsWith('/evaluation-policy-assignments')) {
        return jsonResponse({
          agent_id: 'support-agent',
          environment_id: 'production',
          assignments: JSON.parse(String((init as RequestInit).body)).assignments,
        });
      }
      return jsonResponse({ jobs: [], results: [] });
    });
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const profile = await client.putAgentEvaluationProfile('support-agent', {
      enabled: true,
      capture_mode: 'durable',
      content_mode: 'metadata_only',
      quiet_period_ms: 250n,
      max_capture_wait_ms: 5_000n,
      on_incomplete: 'fail',
    });
    const assignments = await client.putAgentEvaluationPolicyAssignments('support-agent', {
      assignments: [
        {
          policy_id: 'no-denials',
          weight: 1,
          critical: true,
          enabled: true,
        },
      ],
    });
    const evaluations = await client.listRunEvaluations(RUN_BODY.id);

    expect(profile.capture_mode).toBe('durable');
    expect(assignments.assignments[0]?.policy_id).toBe('no-denials');
    expect(evaluations).toEqual({ jobs: [], results: [] });
    expect(fetchSpy.mock.calls.map(([url]) => url)).toEqual([
      'http://server.test/v1/agents/support-agent/evaluation-profile',
      'http://server.test/v1/agents/support-agent/evaluation-policy-assignments',
      `http://server.test/v1/runs/${RUN_BODY.id}/evaluations`,
    ]);
  });
});
