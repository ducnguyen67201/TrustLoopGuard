import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { hardenJob as hardenJobHelper, HardenResponse } from '@/lib/redteam-harden';
import type { setPoliciesEnabled as setPoliciesEnabledHelper } from '@/lib/policies';
import type { RedteamAttackSession } from '@/lib/redteam-jobs';

import { HardenJobCard } from './harden-job-card';

const mockState = vi.hoisted(() => ({
  hardenJob: vi.fn<typeof hardenJobHelper>(),
  setPoliciesEnabled: vi.fn<typeof setPoliciesEnabledHelper>(),
}));

vi.mock('@/lib/redteam-harden', () => ({
  hardenJob: (...args: Parameters<typeof mockState.hardenJob>) => mockState.hardenJob(...args),
}));

vi.mock('@/lib/policies', () => ({
  setPoliciesEnabled: (...args: Parameters<typeof mockState.setPoliciesEnabled>) =>
    mockState.setPoliciesEnabled(...args),
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

const REFUND_SESSION: RedteamAttackSession = {
  ...LANDED_SESSION,
  goal: 'The agent asks for refund destination details before verification.',
  events: [
    {
      event_id: 'event-1',
      seq: 1,
      kind: 'target_reply',
      actor: 'target',
      payload: {},
      content_text: 'Please provide the destination account so I can continue the refund.',
      created_at: '2026-06-26T00:00:00Z',
    },
  ],
};

function response(overrides: Partial<HardenResponse>): HardenResponse {
  return {
    candidates: [],
    rejections: [],
    unreachable: [],
    generated_at: '2026-06-26T00:00:00Z',
    ...overrides,
  };
}

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
    mockState.hardenJob.mockResolvedValue(
      response({
        rejections: [
          {
            reason: 'semantic_judge_unavailable',
            substrate: 'semantic_output',
            evidence_seqs: [0],
            message: 'semantic policy judge is not configured',
          },
          {
            reason: 'missed_variant',
            substrate: 'semantic_output',
            evidence_seqs: [1],
            message: 'candidate missed a reworded version',
            verify: {
              blocked_landed: 3,
              landed_total: 3,
              blocked_variants: 2,
              variant_total: 3,
              false_blocks: 1,
              control_total: 4,
              passed: false,
            },
          },
        ],
      }),
    );

    render(
      <HardenJobCard jobId="job-1" sessions={[REFUND_SESSION]} busy={false} onHardened={vi.fn()} />,
    );

    await userEvent.click(screen.getByRole('button', { name: /build a fix/i }));

    await waitFor(() => expect(screen.getByText(/couldn't verify it/i)).toBeInTheDocument());
    expect(screen.getByText(/semantic policy judge is not configured/i)).toBeInTheDocument();
    expect(screen.getByText(/candidate missed a reworded version/i)).toBeInTheDocument();
    expect(
      screen.getByText(/checked: 3\/3 landed, 2\/3 variants, 1\/4 benign controls blocked/i),
    ).toBeInTheDocument();
    const createRuleHref = screen.getByRole('link', { name: /create rule/i }).getAttribute('href');
    expect(createRuleHref).toContain('/policies/new?');
    expect(createRuleHref).toContain('workspace=test-BJ-V');
    expect(createRuleHref).toContain('environment=production');
    expect(createRuleHref).toContain('policyKey=');
    expect(createRuleHref).toContain('sourceYaml=');
    expect(createRuleHref).toContain('severity=high');
    expect(createRuleHref).toContain('action=deny');
    const sourceYaml = new URL(createRuleHref ?? '', 'http://localhost').searchParams.get(
      'sourceYaml',
    );
    expect(sourceYaml).toContain('semantic:');
    expect(sourceYaml).not.toContain('regex:');
    expect(screen.queryByRole('button', { name: /build a fix/i })).not.toBeInTheDocument();
  });

  it('labels an existing-policy candidate as a tightening', async () => {
    const onHardened = vi.fn();
    mockState.hardenJob.mockResolvedValue(
      response({
        candidates: [
          {
            policy: {
              id: 'harden-agent-1-credential',
              description: 'Blocks credential disclosure.',
              severity: 'critical',
              enabled: true,
              source_yaml: 'id: harden-agent-1-credential\n',
            },
            operation: 'tighten',
            existing_policy_id: 'harden-agent-1-credential',
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
      }),
    );

    render(
      <HardenJobCard
        jobId="job-1"
        sessions={[LANDED_SESSION]}
        busy={false}
        onHardened={onHardened}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /build a fix/i }));

    expect(await screen.findByText('Tightens existing guardrail')).toBeInTheDocument();
    expect(screen.getByText('Tighten')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /test again/i }));
    expect(mockState.setPoliciesEnabled).not.toHaveBeenCalled();
    expect(onHardened).toHaveBeenCalledOnce();
  });
});
