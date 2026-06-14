import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { BenchReportPayload, BenchRunDetail, BenchRunSummary } from '@/lib/bench-jobs';

import { BenchPanel } from './bench-panel';

const mockState = vi.hoisted(() => ({
  createRun: vi.fn<() => Promise<BenchRunDetail>>(),
  getRun: vi.fn<() => Promise<BenchRunDetail>>(),
  listRuns: vi.fn<() => Promise<BenchRunSummary[]>>(),
  getReport: vi.fn<() => Promise<BenchReportPayload>>(),
  cancel: vi.fn<() => Promise<BenchRunSummary>>(),
}));

vi.mock('@/lib/bench-jobs', async () => {
  const actual = await vi.importActual<typeof import('@/lib/bench-jobs')>('@/lib/bench-jobs');
  return {
    ...actual,
    bench: {
      createRun: mockState.createRun,
      getRun: mockState.getRun,
      listRuns: mockState.listRuns,
      getReport: mockState.getReport,
      cancel: mockState.cancel,
    },
  };
});

const RUN: BenchRunSummary = {
  id: 'bench_1',
  workspace_id: 'ws',
  environment_id: 'env',
  status: 'queued',
  profile: 'fast',
  generator: 'deterministic',
  agent_id: null,
  seed: null,
  error: null,
  created_at: '2026-06-14T00:00:00Z',
  updated_at: '2026-06-14T00:00:00Z',
};

const DETAIL: BenchRunDetail = {
  run: RUN,
  arms: [
    {
      run_id: RUN.id,
      arm: 'raw',
      label: 'raw',
      target: 'http://127.0.0.1:9101',
      redteam_job_id: 'raw_job',
      checker_config: 'off',
      created_at: RUN.created_at,
      updated_at: RUN.updated_at,
    },
    {
      run_id: RUN.id,
      arm: 'guarded',
      label: 'guarded',
      target: 'http://127.0.0.1:9102',
      redteam_job_id: 'guarded_job',
      checker_config: 'enforce',
      created_at: RUN.created_at,
      updated_at: RUN.updated_at,
    },
  ],
  raw_job: null,
  guarded_job: null,
};

const COMPLETE_DETAIL: BenchRunDetail = {
  ...DETAIL,
  run: { ...RUN, status: 'complete' },
};

const REPORT: BenchReportPayload = {
  run: COMPLETE_DETAIL.run,
  arms: DETAIL.arms,
  raw: {
    arm: 'raw',
    attacks: 4,
    landed: 3,
    blocked: 0,
    clean: 2,
    errored: 0,
    attack_success_rate: 0.75,
    benign_utility_rate: 1,
    utility_under_attack_rate: 1,
    false_block_rate: 0,
  },
  guarded: {
    arm: 'guarded',
    attacks: 4,
    landed: 0,
    blocked: 3,
    clean: 2,
    errored: 0,
    attack_success_rate: 0,
    benign_utility_rate: 1,
    utility_under_attack_rate: 1,
    false_block_rate: 0,
  },
  delta: {
    attack_success_rate_reduction: 0.75,
    benign_utility_delta: 0,
    utility_under_attack_delta: 0,
    false_block_delta: 0,
  },
  tracks: [],
  cases: [
    {
      case_id: 'case_1',
      attack: 'private-data-exfil',
      goal: 'extract a protected customer token',
      track: 'private_data',
      kind: 'attack',
      raw_outcome: 'landed',
      guarded_outcome: 'blocked',
      status: 'fixed',
    },
  ],
  generated_at: '2026-06-14T00:00:01Z',
};

describe('BenchPanel', () => {
  beforeEach(() => {
    mockState.createRun.mockReset().mockResolvedValue(DETAIL);
    mockState.getRun.mockReset().mockResolvedValue(COMPLETE_DETAIL);
    mockState.listRuns.mockReset().mockResolvedValue([]);
    mockState.getReport.mockReset().mockResolvedValue(REPORT);
    mockState.cancel.mockReset();
  });

  afterEach(() => {
    cleanup();
  });

  it('creates a two-arm benchmark run and renders the Rust-derived report', async () => {
    render(<BenchPanel />);

    await userEvent.click(screen.getByRole('button', { name: /run benchmark/i }));

    await screen.findByText('75%');
    expect(mockState.createRun).toHaveBeenCalledWith({
      rawTargetUrl: 'http://127.0.0.1:9101',
      guardedTargetUrl: 'http://127.0.0.1:9102',
      profile: 'fast',
      generator: 'deterministic',
    });
    expect(screen.getByText('private-data-exfil')).toBeInTheDocument();
    expect(screen.getByText('raw_job')).toBeInTheDocument();
    expect(screen.getByText('guarded_job')).toBeInTheDocument();
  });

  it('clears a finished report when a target URL changes', async () => {
    const user = userEvent.setup();
    render(<BenchPanel />);

    await user.click(screen.getByRole('button', { name: /run benchmark/i }));
    await screen.findByText('private-data-exfil');

    await user.type(screen.getByLabelText('Raw target URL'), '0');

    await waitFor(() => expect(screen.queryByText('private-data-exfil')).not.toBeInTheDocument());
    expect(screen.getByText('No benchmark selected')).toBeInTheDocument();
  });

  it('loads a completed run from history and fetches its report', async () => {
    mockState.listRuns.mockResolvedValue([{ ...RUN, status: 'complete' }]);
    render(<BenchPanel />);

    await userEvent.click(await screen.findByRole('button', { name: /bench_1/i }));

    await screen.findByText('private-data-exfil');
    expect(mockState.getRun).toHaveBeenCalledWith('bench_1');
    expect(mockState.getReport).toHaveBeenCalledWith('bench_1');
  });
});
