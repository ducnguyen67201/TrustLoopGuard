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
          decision: 'block',
          elapsed_ms: 12,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          payload: {
            reason: 'blocked unsafe output',
            triggered_policies: [{ id: 'policy-1' }],
            safe_output: 'Blocked by TrustLoopGuard.',
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
      phase: 'Gateway Output Check',
      verdict: 'Block',
      policy: 'policy-1',
      safeOutput: 'Blocked by TrustLoopGuard.',
    });
  });

  it('rejects malformed run detail payloads', () => {
    expect(() => parseRunDetailSnapshot({ run: null, events: [], traces: [] })).toThrow(
      /run detail contract/,
    );
  });
});
