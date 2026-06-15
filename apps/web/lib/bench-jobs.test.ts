import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { bench, benchRunDetailSchema, benchRunSummarySchema } from './bench-jobs';

const fetchMock = vi.fn<typeof fetch>();

const RUN = {
  id: 'run_1',
  workspace_id: 'ws',
  environment_id: 'env',
  status: 'queued',
  profile: 'fast',
  agent_id: null,
  seed: null,
  error: null,
  created_at: '2026-06-14T00:00:00Z',
  updated_at: '2026-06-14T00:00:00Z',
};

const DETAIL = {
  run: RUN,
  arms: [
    {
      run_id: 'run_1',
      arm: 'raw',
      label: 'raw',
      target: 'http://127.0.0.1:9101',
      redteam_job_id: 'raw-job',
      checker_config: 'off',
      created_at: '2026-06-14T00:00:00Z',
      updated_at: '2026-06-14T00:00:00Z',
    },
    {
      run_id: 'run_1',
      arm: 'guarded',
      label: 'guarded',
      target: 'http://127.0.0.1:9102',
      redteam_job_id: 'guarded-job',
      checker_config: 'enforce',
      created_at: '2026-06-14T00:00:00Z',
      updated_at: '2026-06-14T00:00:00Z',
    },
  ],
  raw_job: null,
  guarded_job: null,
};

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

describe('bench schemas', () => {
  it('accepts a well-formed run summary and detail', () => {
    expect(benchRunSummarySchema.safeParse(RUN).success).toBe(true);
    expect(benchRunDetailSchema.safeParse(DETAIL).success).toBe(true);
  });

  it('rejects an unknown run status', () => {
    expect(benchRunSummarySchema.safeParse({ ...RUN, status: 'paused' }).success).toBe(false);
  });
});

describe('bench client', () => {
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('createRun posts snake_case Rust body and parses detail', async () => {
    fetchMock.mockResolvedValue(jsonResponse(DETAIL, 201));

    const detail = await bench.createRun({
      rawTargetUrl: 'http://127.0.0.1:9101',
      guardedTargetUrl: 'http://127.0.0.1:9102',
      profile: 'fast',
    });

    expect(detail.run.id).toBe('run_1');
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe('/api/bench/runs');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(String(init?.body))).toEqual({
      raw_target_url: 'http://127.0.0.1:9101',
      guarded_target_url: 'http://127.0.0.1:9102',
      profile: 'fast',
    });
  });

  it('listRuns forwards limit and returns the runs array', async () => {
    fetchMock.mockResolvedValue(jsonResponse({ runs: [RUN] }));

    const runs = await bench.listRuns({ limit: 5 });

    expect(runs).toHaveLength(1);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/bench/runs?limit=5');
  });

  it('getReport reads the Rust-derived report endpoint', async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({
        run: { ...RUN, status: 'complete' },
        arms: DETAIL.arms,
        raw: {
          arm: 'raw',
          attacks: 1,
          landed: 1,
          blocked: 0,
          clean: 0,
          errored: 0,
          attack_success_rate: 1,
          benign_utility_rate: 0,
          utility_under_attack_rate: 0,
          false_block_rate: 0,
        },
        guarded: {
          arm: 'guarded',
          attacks: 1,
          landed: 0,
          blocked: 1,
          clean: 0,
          errored: 0,
          attack_success_rate: 0,
          benign_utility_rate: 0,
          utility_under_attack_rate: 0,
          false_block_rate: 0,
        },
        delta: {
          attack_success_rate_reduction: 1,
          benign_utility_delta: 0,
          utility_under_attack_delta: 0,
          false_block_delta: 0,
        },
        tracks: [],
        cases: [],
        generated_at: '2026-06-14T00:00:01Z',
      }),
    );

    const report = await bench.getReport('run_1');

    expect(report.delta.attack_success_rate_reduction).toBe(1);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/bench/runs/run_1/report');
  });
});
