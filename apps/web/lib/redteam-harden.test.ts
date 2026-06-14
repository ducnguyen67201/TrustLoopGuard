import { afterEach, describe, expect, it, vi } from 'vitest';

import { hardenJob } from './redteam-harden';

function candidateResponse(): string {
  return JSON.stringify({
    candidates: [
      {
        policy: {
          id: 'harden-credential',
          description: 'Blocks replies that disclose an API key or credential.',
          severity: 'critical',
          enabled: false,
          source_yaml: 'id: harden-credential\n',
        },
        substrate: 'semantic_output',
        evidence_seqs: [0],
        source: 'deterministic',
        verify: {
          blocked_landed: 1,
          landed_total: 1,
          blocked_variants: 2,
          variant_total: 2,
          false_blocks: 0,
          control_total: 0,
          passed: true,
        },
      },
    ],
    unreachable: [],
    generated_at: '2026-06-14T00:00:00Z',
  });
}

describe('hardenJob', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    window.history.replaceState({}, '', '/');
  });

  it('posts to the job harden endpoint and parses verified candidates', async () => {
    const fetchMock = vi.fn<typeof fetch>(
      async () =>
        new Response(candidateResponse(), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        }),
    );
    vi.stubGlobal('fetch', fetchMock);

    const result = await hardenJob('job-1', true);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/redteam/jobs/job-1/harden',
      expect.objectContaining({ method: 'POST' }),
    );
    expect(result.candidates).toHaveLength(1);
    const candidate = result.candidates[0];
    expect(candidate).toBeDefined();
    expect(candidate?.policy.enabled).toBe(false);
    expect(candidate?.substrate).toBe('semantic_output');
    expect(candidate?.verify.passed).toBe(true);
  });

  it('preserves the selected workspace on the harden request', async () => {
    window.history.replaceState({}, '', '/attacks?workspace=trustloopguard-demo');
    const fetchMock = vi.fn<typeof fetch>(
      async () =>
        new Response(
          JSON.stringify({ candidates: [], unreachable: [], generated_at: '2026-06-14T00:00:00Z' }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        ),
    );
    vi.stubGlobal('fetch', fetchMock);

    await hardenJob('job-1', false);

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/redteam/jobs/job-1/harden?workspace=trustloopguard-demo',
      expect.objectContaining({ method: 'POST' }),
    );
  });
});
