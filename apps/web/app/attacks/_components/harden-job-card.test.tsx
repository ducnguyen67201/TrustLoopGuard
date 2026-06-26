import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { HardenResponse } from '@/lib/redteam-harden';
import type { RedteamAttackSession } from '@/lib/redteam-jobs';

import { HardenJobCard } from './harden-job-card';

const hardenJob = vi.fn<() => Promise<HardenResponse>>();

vi.mock('@/lib/redteam-harden', () => ({
  hardenJob: () => hardenJob(),
}));

const LANDED_SESSION: RedteamAttackSession = {
  session_id: 'session-1',
  seq: 0,
  attack: 'prompt-leak',
  goal: 'extract a secret',
  status: 'complete',
  outcome: 'landed',
  landed: true,
  events: [],
};

describe('HardenJobCard', () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    window.history.replaceState({}, '', '/');
  });

  it('hands off to policy authoring when no verified candidate survives', async () => {
    window.history.replaceState(
      {},
      '',
      '/attacks?workspace=test-BJ-V&environment=production&id=job-1',
    );
    hardenJob.mockResolvedValue({
      candidates: [],
      unreachable: ['semantic_output'],
      generated_at: '2026-06-26T00:00:00Z',
    });

    render(
      <HardenJobCard
        jobId="job-1"
        sessions={[LANDED_SESSION]}
        busy={false}
        onHardened={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /build a fix/i }));

    await waitFor(() => expect(screen.getByText(/couldn't auto-build/i)).toBeInTheDocument());
    expect(screen.getByText(/Missing coverage: semantic output/i)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /create rule/i })).toHaveAttribute(
      'href',
      '/policies/new?workspace=test-BJ-V&environment=production',
    );
    expect(screen.queryByRole('button', { name: /build a fix/i })).not.toBeInTheDocument();
  });
});
