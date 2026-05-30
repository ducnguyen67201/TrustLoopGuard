import { describe, expect, it } from 'vitest';

import { parseRunsSnapshot } from './runs-live';

describe('parseRunsSnapshot', () => {
  it('validates and maps a runs list payload into table rows', () => {
    const rows = parseRunsSnapshot(
      {
        runs: [
          {
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
            trace_count: 3,
            blocked_count: 2,
            rewritten_count: 0,
            escalated_count: 1,
            p95_latency_ms: 12,
          },
        ],
      },
      'demo-workspace',
    );

    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      shortId: '019f0000...0001',
      agent: 'demo-agent',
      kind: 'Chat Session',
      status: 'Running',
      externalId: 'arena-session-1',
      traces: 3,
      blocked: 2,
      escalated: 1,
      latency: '12ms',
      href: '/runs/019f0000-0000-7000-9000-000000000001?workspace=demo-workspace',
    });
  });

  it('renders a placeholder latency when a run has no traces yet', () => {
    const rows = parseRunsSnapshot(
      {
        runs: [
          {
            id: 'run-2',
            workspace_id: 'ws_demo',
            agent_id: 'demo-agent',
            kind: 'chat_session',
            status: 'running',
            external_id: null,
            metadata: {},
            started_at: '2026-05-25T00:00:00.000Z',
            ended_at: null,
            created_at: '2026-05-25T00:00:00.000Z',
            updated_at: '2026-05-25T00:00:00.000Z',
            trace_count: 0,
            blocked_count: 0,
            rewritten_count: 0,
            escalated_count: 0,
            p95_latency_ms: null,
          },
        ],
      },
      'demo-workspace',
    );

    expect(rows[0]).toMatchObject({ latency: 'No traces', externalId: 'None' });
  });

  it('rejects malformed runs payloads', () => {
    expect(() => parseRunsSnapshot({ runs: null }, 'demo-workspace')).toThrow(
      /runs list contract/,
    );
  });
});
