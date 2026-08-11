import { renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { useActivationRun } from './useActivationRun';

describe('useActivationRun', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('binds verification to the exact environment/session and waits for finalization/evaluation', async () => {
    const run = {
      id: 'run-exact',
      status: 'completed',
      external_id: 'verify-exact',
    };
    const fetchMock = vi.fn<typeof fetch>(async (input) => {
      const url = String(input);
      if (url.startsWith('/api/runs?')) {
        return new Response(JSON.stringify({ runs: [run] }), { status: 200 });
      }
      return new Response(
        JSON.stringify({
          run,
          finalization: {
            finalized_at: '2026-08-11T00:00:00Z',
            boundary_source: 'framework_adapter',
            boundary_confidence: 'strong',
            capture_status: 'complete',
            capture_deadline: '2026-08-11T00:00:01Z',
            expected_flush_id: null,
          },
          evaluations: [{ verdict: 'passed' }],
          evaluation_jobs: [{ status: 'completed' }],
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal('fetch', fetchMock);

    const { result } = renderHook(() => useActivationRun('verify-exact', 'workspace-1', 'staging'));

    await waitFor(() => expect(result.current.evaluationComplete).toBe(true));
    expect(result.current.run?.id).toBe('run-exact');
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const listCall = fetchMock.mock.calls[0];
    const detailCall = fetchMock.mock.calls[1];
    if (listCall === undefined || detailCall === undefined) {
      throw new Error('verification requests missing');
    }
    expect(String(listCall[0])).toContain('environment=staging');
    expect(String(listCall[0])).toContain('external_id=verify-exact');
    expect(String(detailCall[0])).toContain('/api/runs/run-exact?');
  });
});
