import { describe, expect, it } from 'vitest';

import { currentAssuranceStatus, parseRunDetailSnapshot } from './run-detail-live';

describe('parseRunDetailSnapshot', () => {
  it('validates and maps a run detail payload for the live view', () => {
    const snapshot = parseRunDetailSnapshot({
      run: {
        id: '019f0000-0000-7000-9000-000000000001',
        workspace_id: 'ws_demo',
        agent_id: 'demo-agent',
        kind: 'chat_session',
        status: 'running',
        external_id: 'arena-session-1',
        metadata: { route_id: 'route-1' },
        started_at: '2026-05-25T00:00:00.000Z',
        ended_at: null,
        created_at: '2026-05-25T00:00:00.000Z',
        updated_at: '2026-05-25T00:00:01.000Z',
        trace_count: 2,
        blocked_count: 1,
        rewritten_count: 0,
        escalated_count: 0,
        p95_latency_ms: 12,
      },
      events: [
        {
          id: 'event-1',
          workspace_id: 'ws_demo',
          run_id: '019f0000-0000-7000-9000-000000000001',
          agent_id: 'demo-agent',
          sequence: 1,
          kind: 'user_turn',
          label: null,
          input_summary: 'hello',
          output_summary: 'hi',
          metadata: {},
          occurred_at: '2026-05-25T00:00:00.000Z',
          created_at: '2026-05-25T00:00:00.000Z',
        },
      ],
      traces: [
        {
          trace_id: 'trace-1',
          run_id: '019f0000-0000-7000-9000-000000000001',
          run_event_id: 'event-1',
          domain: 'gateway_output_check',
          decision: 'deny',
          elapsed_ms: 12,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          payload: {
            reason: 'blocked unsafe output',
            triggered_policies: [{ id: 'policy-1' }],
            safe_output: 'Blocked by Featherlane AI.',
            checked_output_excerpt: 'That is a stupid question. Figure it out yourself.',
          },
          created_at: '2026-05-25T00:00:01.000Z',
        },
      ],
      finalization: {
        finalized_at: '2026-05-25T00:00:02.000Z',
        boundary_source: 'explicit_sdk',
        boundary_confidence: 'authoritative',
        capture_status: 'complete',
        capture_deadline: '2026-05-25T00:00:32.000Z',
        expected_flush_id: null,
      },
      participants: [
        { agent_id: 'demo-agent', role: 'primary', joined_at: '2026-05-25T00:00:00.000Z' },
      ],
      evaluation_jobs: [
        {
          id: 'job-1',
          run_id: '019f0000-0000-7000-9000-000000000001',
          agent_id: 'demo-agent',
          status: 'completed',
          attempts: 1,
          error: null,
          updated_at: '2026-05-25T00:00:03.000Z',
        },
      ],
      evaluations: [
        {
          id: 'eval-1',
          run_id: '019f0000-0000-7000-9000-000000000001',
          agent_id: 'demo-agent',
          snapshot_hash: 'blake3:v1:snapshot',
          manifest_hash: 'blake3:v1:manifest',
          evaluator_version: 'tl-eval:v1',
          verdict: 'failed',
          score_bps: 0,
          capture_status: 'complete',
          created_at: '2026-05-25T00:00:03.000Z',
        },
      ],
    });

    expect(snapshot.run).toMatchObject({
      agent: 'demo-agent',
      kind: 'Chat Session',
      status: 'Running',
      traces: 2,
      blocked: 1,
      latency: '12ms',
    });
    expect(snapshot.events[0]).toMatchObject({
      kind: 'User Turn',
      label: 'Turn 1',
      input: 'hello',
      output: 'hi',
    });
    expect(snapshot.traces[0]).toMatchObject({
      runEventId: 'event-1',
      side: 'output',
      phase: 'Gateway Output Check',
      effect: 'Deny',
      outcome: 'deny',
      triggered: true,
      policy: 'policy-1',
      safeOutput: 'Blocked by Featherlane AI.',
      checkedOutput: 'That is a stupid question. Figure it out yourself.',
    });
    expect(snapshot.assurance.finalization?.capture_status).toBe('complete');
    expect(snapshot.assurance.eligibility).toBe('eligible');
    expect(snapshot.assurance.participants).toHaveLength(1);
    expect(snapshot.assurance.jobs[0]).toMatchObject({ status: 'completed', attempts: 1 });
    expect(snapshot.assurance.evaluations[0]).toMatchObject({ verdict: 'failed', score_bps: 0 });
  });

  it('preserves legacy-incomplete evaluation eligibility', () => {
    const snapshot = parseRunDetailSnapshot({
      run: {
        id: 'legacy-run',
        workspace_id: 'ws_demo',
        agent_id: 'demo-agent',
        kind: 'chat_session',
        status: 'completed',
        evaluation_eligibility: 'legacy_incomplete',
        external_id: null,
        metadata: {},
        started_at: '2026-05-25T00:00:00.000Z',
        ended_at: '2026-05-25T00:00:01.000Z',
        created_at: '2026-05-25T00:00:00.000Z',
        updated_at: '2026-05-25T00:00:01.000Z',
        trace_count: 0,
        blocked_count: 0,
        rewritten_count: 0,
        escalated_count: 0,
        p95_latency_ms: null,
      },
      events: [],
      traces: [],
    });

    expect(snapshot.assurance.eligibility).toBe('legacy_incomplete');
  });

  it('uses only the latest evaluation per agent for the current assurance status', () => {
    const assurance = {
      eligibility: 'eligible' as const,
      finalization: null,
      participants: [],
      jobs: [],
      evaluations: [
        {
          id: 'old-failure',
          run_id: 'run',
          agent_id: 'agent',
          snapshot_hash: 'old',
          manifest_hash: 'manifest',
          evaluator_version: 'tl-eval:v1',
          verdict: 'failed',
          score_bps: 0,
          capture_status: 'complete',
          created_at: '2026-01-01T00:00:00Z',
        },
        {
          id: 'new-pass',
          run_id: 'run',
          agent_id: 'agent',
          snapshot_hash: 'new',
          manifest_hash: 'manifest',
          evaluator_version: 'tl-eval:v1',
          verdict: 'passed',
          score_bps: 10_000,
          capture_status: 'complete',
          created_at: '2026-01-02T00:00:00Z',
        },
      ],
    };

    expect(currentAssuranceStatus(assurance)).toBe('passed');
  });

  it('marks untriggered allow checks with their input/output side', () => {
    const snapshot = parseRunDetailSnapshot({
      run: {
        id: '019f0000-0000-7000-9000-000000000002',
        workspace_id: 'ws_demo',
        agent_id: 'demo-agent',
        kind: 'chat_session',
        status: 'running',
        external_id: null,
        metadata: {},
        started_at: '2026-05-25T00:00:00.000Z',
        ended_at: null,
        created_at: '2026-05-25T00:00:00.000Z',
        updated_at: '2026-05-25T00:00:01.000Z',
        trace_count: 1,
        blocked_count: 0,
        rewritten_count: 0,
        escalated_count: 0,
        p95_latency_ms: 5,
      },
      events: [],
      traces: [
        {
          trace_id: 'trace-input',
          run_id: '019f0000-0000-7000-9000-000000000002',
          run_event_id: null,
          domain: 'gateway_input_check',
          decision: 'permit',
          elapsed_ms: 5,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          payload: { reason: 'no policies triggered', checked_input_excerpt: 'user: hi' },
          created_at: '2026-05-25T00:00:01.000Z',
        },
      ],
    });

    expect(snapshot.traces[0]).toMatchObject({
      side: 'input',
      outcome: 'permit',
      triggered: false,
      severity: null,
      policy: 'baseline',
    });
    const inputTrace = snapshot.traces[0];
    expect(inputTrace?.timestamp).toBe(new Date('2026-05-25T00:00:01.000Z').getTime());
    expect(typeof inputTrace?.clock).toBe('string');
    expect(inputTrace?.clock).not.toBe('');
  });

  it('surfaces guarded tool calls from canonical event trace payloads', () => {
    const snapshot = parseRunDetailSnapshot({
      run: {
        id: '019f0000-0000-7000-9000-000000000003',
        workspace_id: 'ws_demo',
        agent_id: 'booking-agent',
        kind: 'chat_session',
        status: 'completed',
        external_id: 'booking-1',
        metadata: {},
        started_at: '2026-05-25T00:00:00.000Z',
        ended_at: '2026-05-25T00:00:02.000Z',
        created_at: '2026-05-25T00:00:00.000Z',
        updated_at: '2026-05-25T00:00:02.000Z',
        trace_count: 1,
        blocked_count: 0,
        rewritten_count: 0,
        escalated_count: 0,
        p95_latency_ms: 8,
      },
      events: [
        {
          id: 'event-3',
          workspace_id: 'ws_demo',
          run_id: '019f0000-0000-7000-9000-000000000003',
          agent_id: 'booking-agent',
          sequence: 1,
          kind: 'assistant_turn',
          label: 'guarded_agent_reply',
          input_summary: 'Book for two people',
          output_summary: null,
          metadata: {},
          occurred_at: '2026-05-25T00:00:01.000Z',
          created_at: '2026-05-25T00:00:01.000Z',
        },
      ],
      traces: [
        {
          trace_id: 'trace-tool',
          run_id: '019f0000-0000-7000-9000-000000000003',
          run_event_id: 'event-3',
          domain: 'event',
          decision: 'permit',
          elapsed_ms: 8,
          payload: {
            reason: 'current policy and authority permit the subject',
            event: {
              kind: 'tool.call.proposed',
              action: {
                operation: 'book_appointment',
                parameters: {
                  customer: 'Browser QA customer',
                  partySize: 2,
                },
                tool_identity: {
                  server_id: 'mastra',
                  tool_name: 'book_appointment',
                  schema_hash: 'featherlane-ai-schema:fnv1a64:test',
                },
              },
            },
          },
          created_at: '2026-05-25T00:00:01.000Z',
        },
      ],
    });

    expect(snapshot.traces[0]).toMatchObject({
      side: 'tool',
      phase: 'Tool Call Proposed',
      operation: 'book_appointment',
      toolName: 'book_appointment',
      checkedInput: expect.stringContaining('"partySize": 2'),
    });
  });

  it('rejects malformed run detail payloads', () => {
    expect(() => parseRunDetailSnapshot({ run: null, events: [], traces: [] })).toThrow(
      /run detail contract/,
    );
  });

  it('groups legacy unlinked gateway traces into synthetic turns', () => {
    const snapshot = parseRunDetailSnapshot({
      run: {
        id: '019f0000-0000-7000-9000-000000000001',
        workspace_id: 'ws_demo',
        agent_id: 'demo-agent',
        kind: 'chat_session',
        status: 'completed',
        external_id: 'arena-session-1',
        metadata: {},
        started_at: '2026-05-25T00:00:00.000Z',
        ended_at: '2026-05-25T00:00:02.000Z',
        created_at: '2026-05-25T00:00:00.000Z',
        updated_at: '2026-05-25T00:00:02.000Z',
        trace_count: 2,
        blocked_count: 1,
        rewritten_count: 0,
        escalated_count: 0,
        p95_latency_ms: 12,
      },
      events: [],
      traces: [
        {
          trace_id: 'trace-input',
          run_id: '019f0000-0000-7000-9000-000000000001',
          run_event_id: null,
          domain: 'gateway_input_check',
          decision: 'permit',
          elapsed_ms: 5,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          payload: {
            reason: 'no policies triggered',
            checked_input_excerpt: 'user: hello\nuser: book an appointment',
          },
          created_at: '2026-05-25T00:00:01.000Z',
        },
        {
          trace_id: 'trace-output',
          run_id: '019f0000-0000-7000-9000-000000000001',
          run_event_id: null,
          domain: 'gateway_output_check',
          decision: 'deny',
          elapsed_ms: 7,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          payload: {
            reason: 'blocked unsafe output',
            triggered_policies: [{ id: 'policy-1' }],
            checked_output_excerpt: 'That is a stupid question. Figure it out yourself.',
          },
          created_at: '2026-05-25T00:00:02.000Z',
        },
      ],
    });

    expect(snapshot.events).toHaveLength(1);
    expect(snapshot.events[0]).toMatchObject({
      kind: 'User Turn',
      input: 'book an appointment',
      output: 'That is a stupid question. Figure it out yourself.',
    });
    const event = snapshot.events[0];
    expect(event).toBeDefined();
    expect(snapshot.traces.every((trace) => trace.runEventId === event?.id)).toBe(true);
  });
});
