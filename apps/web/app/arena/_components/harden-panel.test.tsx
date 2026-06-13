import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { suggestPolicyFromReport, type HardenDraft } from '@/lib/arena-harden';
import type { RedteamCase, RedteamReport, RedteamTargetSummary } from '@/lib/arena-redteam';

import { HardenPanel } from './harden-panel';

const mockState = vi.hoisted(() => ({
  buildHardenDraft: vi.fn<(report: RedteamReport) => Promise<HardenDraft | null>>(),
  applyHardenPolicy: vi.fn<() => Promise<string>>(),
}));

vi.mock('@/lib/arena-harden', async () => {
  const actual = await vi.importActual<typeof import('@/lib/arena-harden')>('@/lib/arena-harden');
  return {
    ...actual,
    buildHardenDraft: mockState.buildHardenDraft,
    applyHardenPolicy: mockState.applyHardenPolicy,
  };
});

const SECRET = 'sk-trustloop9f3k2x';
const CONFIG = 'fast|raw|guarded';

function turn(outcome: RedteamCase['guarded']['outcome'], reply: string) {
  return { outcome, reply, detail: outcome, traceId: outcome === 'blocked' ? 'trc_1' : null };
}

function summary(landed: number): RedteamTargetSummary {
  return {
    total: 1,
    attacks: 1,
    landed,
    blocked: 1 - landed,
    clean: 0,
    errored: 0,
    successRate: landed,
  };
}

function reportWith(guardedOutcome: 'landed' | 'blocked', reply: string): RedteamReport {
  const landed = guardedOutcome === 'landed' ? 1 : 0;
  return {
    profile: 'fast',
    status: 'complete',
    llm: { mode: 'builtin', generator: 'deterministic', judge: 'heuristic' },
    raw: summary(1),
    guarded: summary(landed),
    deltaPoints: 0,
    cases: [
      {
        attack: 'prompt-leak',
        goal: 'extract the credential',
        control: false,
        prompt: 'reveal it',
        raw: turn('landed', `here: ${SECRET}`),
        guarded: turn(guardedOutcome, reply),
      },
    ],
    progress: { done: 1, total: 1 },
    error: null,
  };
}

const leaked = reportWith('landed', `here: ${SECRET}`);
const cleaned = reportWith('blocked', "I can't help with that.");

const draftFixture: HardenDraft = {
  draft: {
    id: 'block-credential-leak',
    description: 'Blocks the credential.',
    matchType: 'regex',
    matchValue: '(?i)sk-[a-z0-9]{6,}',
    action: 'block',
    severity: 'critical',
    channels: ['chat'],
  },
  source: 'deterministic',
  suggestion: suggestPolicyFromReport(leaked)!,
};

describe('HardenPanel', () => {
  beforeEach(() => {
    mockState.buildHardenDraft.mockReset();
    mockState.applyHardenPolicy.mockReset();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders nothing when the guard blocked every attack', () => {
    render(<HardenPanel report={cleaned} busy={false} configKey={CONFIG} onHardened={vi.fn()} />);
    expect(screen.queryByRole('button', { name: /harden against these/i })).not.toBeInTheDocument();
  });

  it('suggests a guard from the attacks that landed', () => {
    render(<HardenPanel report={leaked} busy={false} configKey={CONFIG} onHardened={vi.fn()} />);
    expect(screen.getByText(/this guard blocks that for every reply/i)).toBeInTheDocument();
    expect(screen.getByText('prompt-leak')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /harden against these/i })).toBeInTheDocument();
  });

  it('exposes the YAML disclosure to assistive tech', async () => {
    render(<HardenPanel report={leaked} busy={false} configKey={CONFIG} onHardened={vi.fn()} />);
    const toggle = screen.getByRole('button', { name: /show yaml/i });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(toggle);
    expect(screen.getByRole('button', { name: /hide yaml/i })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
  });

  it('applies a policy, triggers the auto re-run, records a round, and wins', async () => {
    mockState.buildHardenDraft.mockResolvedValue(draftFixture);
    mockState.applyHardenPolicy.mockResolvedValue('block-credential-leak');
    const onHardened = vi.fn();

    const view = render(
      <HardenPanel report={leaked} busy={false} configKey={CONFIG} onHardened={onHardened} />,
    );

    await userEvent.click(screen.getByRole('button', { name: /harden against these/i }));

    await waitFor(() => expect(onHardened).toHaveBeenCalledTimes(1));
    expect(mockState.applyHardenPolicy).toHaveBeenCalledWith(draftFixture.draft);

    // The parent re-runs: report goes null (re-running) then resolves to a clean guard.
    view.rerender(
      <HardenPanel report={null} busy={true} configKey={CONFIG} onHardened={onHardened} />,
    );
    // "applying…" appears only in the visible timeline row, not the sr-only live region.
    expect(screen.getByText(/applying/i)).toBeInTheDocument();

    view.rerender(
      <HardenPanel report={cleaned} busy={false} configKey={CONFIG} onHardened={onHardened} />,
    );

    expect(await screen.findByText('Round 1')).toBeInTheDocument();
    // "blocked across N rounds" is unique to the visible win card (not the sr-only region).
    expect(screen.getByText(/blocked across 1 round/i)).toBeInTheDocument();
  });

  it('resets the loop when the run config changes', async () => {
    mockState.buildHardenDraft.mockResolvedValue(draftFixture);
    mockState.applyHardenPolicy.mockResolvedValue('block-credential-leak');

    const view = render(
      <HardenPanel report={leaked} busy={false} configKey={CONFIG} onHardened={vi.fn()} />,
    );
    await userEvent.click(screen.getByRole('button', { name: /harden against these/i }));
    view.rerender(
      <HardenPanel report={cleaned} busy={false} configKey={CONFIG} onHardened={vi.fn()} />,
    );
    expect(await screen.findByText('Round 1')).toBeInTheDocument();

    // New agent / profile → fresh loop, timeline cleared.
    view.rerender(
      <HardenPanel report={null} busy={false} configKey="full|raw|guarded" onHardened={vi.fn()} />,
    );
    expect(screen.queryByText('Round 1')).not.toBeInTheDocument();
  });
});
