import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { RedteamJobSummary, RedteamReportShare } from '@/lib/redteam-jobs';

import { ReportShareCard } from './report-share-card';

const mockState = vi.hoisted(() => ({
  listJobs: vi.fn<() => Promise<RedteamJobSummary[]>>(),
  createReport: vi.fn<() => Promise<RedteamReportShare>>(),
  revokeReport: vi.fn<() => Promise<void>>(),
}));

vi.mock('@/lib/redteam-jobs', async () => {
  const actual = await vi.importActual<typeof import('@/lib/redteam-jobs')>('@/lib/redteam-jobs');
  return {
    ...actual,
    redteam: {
      listJobs: mockState.listJobs,
      createReport: mockState.createReport,
      revokeReport: mockState.revokeReport,
    },
  };
});

const JOB: RedteamJobSummary = {
  id: 'job_1',
  workspace_id: 'ws',
  environment_id: 'env',
  status: 'complete',
  target: 'http://127.0.0.1:9101',
  profile: 'fast',
  generator: 'deterministic',
  agent_id: 'agent-1',
  attacks: 4,
  landed: 3,
  blocked: 1,
  error: null,
  created_at: '2026-06-13T00:00:00Z',
  updated_at: '2026-06-13T00:00:00Z',
};

const SHARE: RedteamReportShare = {
  token: 'rpt_test123',
  path: '/r/rpt_test123',
  job_id: 'job_1',
  compare_job_id: null,
  created_at: '2026-06-13T00:00:00Z',
  expires_at: '2026-07-13T00:00:00Z',
};

describe('ReportShareCard', () => {
  beforeEach(() => {
    mockState.listJobs.mockReset().mockResolvedValue([]);
    mockState.createReport.mockReset().mockResolvedValue(SHARE);
    mockState.revokeReport.mockReset().mockResolvedValue();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('mints a shareable link and shows the absolute URL', async () => {
    const user = userEvent.setup();
    render(<ReportShareCard job={JOB} />);

    await user.click(screen.getByRole('button', { name: /create report link/i }));
    expect(await screen.findByText('Create shareable report')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /^create link$/i }));

    await waitFor(() =>
      expect(mockState.createReport).toHaveBeenCalledWith({ jobId: 'job_1', ttlDays: 30 }),
    );
    const openLink = await screen.findByRole('link', { name: /open report/i });
    expect(openLink).toHaveAttribute('href', expect.stringContaining('/r/rpt_test123'));
    expect(screen.getByLabelText('Shareable report link').textContent).toContain('/r/rpt_test123');
  });

  it('offers only same-agent completed runs as comparison candidates', async () => {
    mockState.listJobs.mockResolvedValue([
      { ...JOB, id: 'job_2', agent_id: 'agent-1', status: 'complete' }, // eligible
      { ...JOB, id: 'job_3', agent_id: 'agent-2', status: 'complete' }, // different agent
      { ...JOB, id: 'job_4', agent_id: 'agent-1', status: 'running' }, // not complete
    ]);
    const user = userEvent.setup();
    render(<ReportShareCard job={JOB} />);

    await user.click(screen.getByRole('button', { name: /create report link/i }));

    await waitFor(() => expect(mockState.listJobs).toHaveBeenCalled());
    // One eligible run exists, so the "no other completed runs" hint must not show.
    await waitFor(() =>
      expect(screen.queryByText(/no other completed runs/i)).not.toBeInTheDocument(),
    );
  });

  it('shows the empty-comparison hint when no same-agent runs exist', async () => {
    mockState.listJobs.mockResolvedValue([
      { ...JOB, id: 'job_x', agent_id: 'agent-9', status: 'complete' }, // different agent
    ]);
    const user = userEvent.setup();
    render(<ReportShareCard job={JOB} />);

    await user.click(screen.getByRole('button', { name: /create report link/i }));

    expect(await screen.findByText(/no other completed runs/i)).toBeInTheDocument();
  });
});
