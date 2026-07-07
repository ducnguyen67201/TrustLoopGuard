import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LlmUsageBucketsResponse } from '@trustloopguard/sdk';

vi.mock('server-only', () => ({}));

const rustApiForWorkspace =
  vi.fn<(workspaceId: string, path: string, init: unknown) => Promise<LlmUsageBucketsResponse>>();

vi.mock('@/lib/server/tl-client', () => ({
  rustApiForWorkspace: (workspaceId: string, path: string, init: unknown) =>
    rustApiForWorkspace(workspaceId, path, init),
}));

import { loadUsageBuckets } from './loader';

const WS = 'ws_1';
const NOW = new Date('2026-07-08T12:00:00Z'); // a Wednesday

beforeEach(() => {
  rustApiForWorkspace.mockReset();
  rustApiForWorkspace.mockImplementation(async (_ws, path) => ({
    buckets: [{ key: path, prompt_tokens: 0n, completion_tokens: 0n, cost_minor: 0n, calls: 0n }],
  }));
});

describe('loadUsageBuckets', () => {
  it('fetches all three group_by rollups for the resolved week window in one call', async () => {
    const result = await loadUsageBuckets(WS, { period: 'week', now: NOW });

    expect(rustApiForWorkspace).toHaveBeenCalledTimes(3);
    const paths = rustApiForWorkspace.mock.calls.map((call) => call[1]);
    expect(paths.some((p) => p.includes('group_by=day'))).toBe(true);
    expect(paths.some((p) => p.includes('group_by=principal'))).toBe(true);
    expect(paths.some((p) => p.includes('group_by=model'))).toBe(true);
    // Week starts Monday 2026-07-06 UTC, matching the budget engine.
    for (const path of paths) {
      expect(path).toContain(`start=${encodeURIComponent('2026-07-06T00:00:00.000Z')}`);
      expect(path).toContain(`end=${encodeURIComponent('2026-07-13T00:00:00.000Z')}`);
    }
    expect(result.period).toBe('week');
    expect(result.principalId).toBeNull();
  });

  it('only fetches the requested groups (compact embeds skip the day series)', async () => {
    const result = await loadUsageBuckets(WS, { period: 'week', groups: ['principal'], now: NOW });

    expect(rustApiForWorkspace).toHaveBeenCalledTimes(1);
    expect(rustApiForWorkspace.mock.calls[0]![1]).toContain('group_by=principal');
    expect(result.day).toEqual([]);
    expect(result.model).toEqual([]);
    expect(result.principal).toHaveLength(1);
  });

  it('threads a principal_id filter and records the focus', async () => {
    const result = await loadUsageBuckets(WS, {
      period: 'week',
      principalId: 'refund-bot',
      now: NOW,
    });

    for (const call of rustApiForWorkspace.mock.calls) {
      expect(call[1]).toContain('principal_id=refund-bot');
    }
    expect(result.principalId).toBe('refund-bot');
  });

  it('degrades to empty buckets when a group fails rather than throwing', async () => {
    rustApiForWorkspace.mockRejectedValueOnce(new Error('boom'));
    const result = await loadUsageBuckets(WS, { period: 'day', now: NOW });

    // The failed group is empty; the page still renders.
    const emptyGroups = [result.day, result.principal, result.model].filter(
      (group) => group.length === 0,
    );
    expect(emptyGroups.length).toBeGreaterThanOrEqual(1);
  });
});
