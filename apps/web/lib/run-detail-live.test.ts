import { describe, expect, it } from 'vitest';

import { parseRunDetailSnapshot } from './run-detail-live';

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
            safe_output: 'Blocked by TrustLoopGuard.',
            checked_output_excerpt: 'That is a stupid question. Figure it out yourself.',
          },
          created_at: '2026-05-25T00:00:01.000Z',
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
      safeOutput: 'Blocked by TrustLoopGuard.',
      checkedOutput: 'That is a stupid question. Figure it out yourself.',
    });
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
