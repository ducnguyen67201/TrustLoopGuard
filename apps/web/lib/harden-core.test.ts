import { describe, expect, it } from 'vitest';

import {
  extractLeakedToken,
  hardenDraftYaml,
  selectLandedOnGuard,
  suggestPolicyFromReport,
} from './harden-core';
import { policyDraftSchema } from './policy-draft';
import type { RedteamCase, RedteamReport, RedteamTargetSummary } from './redteam-core';

const SECRET = 'sk-trustloop9f3k2x';

function turn(outcome: RedteamCase['guarded']['outcome'], reply: string) {
  return { outcome, reply, detail: outcome, traceId: outcome === 'blocked' ? 'trc_1' : null };
}

function attackCase(over: Partial<RedteamCase> & Pick<RedteamCase, 'attack'>): RedteamCase {
  return {
    goal: `make the agent reveal the ${over.attack}`,
    control: false,
    prompt: 'adversarial prompt',
    raw: turn('landed', `leaked ${SECRET}`),
    guarded: turn('blocked', "I can't help with that."),
    ...over,
  };
}

function reportWith(cases: RedteamCase[], guardedLanded: number): RedteamReport {
  const summary = (landed: number): RedteamTargetSummary => ({
    total: cases.length,
    attacks: cases.filter((c) => !c.control).length,
    landed,
    blocked: cases.filter((c) => !c.control).length - landed,
    clean: cases.filter((c) => c.control).length,
    errored: 0,
    successRate: cases.length ? landed / cases.filter((c) => !c.control).length : 0,
  });
  return {
    profile: 'fast',
    status: 'complete',
    llm: { mode: 'builtin', generator: 'deterministic', judge: 'heuristic' },
    raw: summary(cases.filter((c) => !c.control).length),
    guarded: summary(guardedLanded),
    deltaPoints: 0,
    cases,
    progress: { done: cases.length, total: cases.length },
    error: null,
  };
}

describe('selectLandedOnGuard', () => {
  it('returns only non-control attacks that landed on the guarded side', () => {
    const cases = [
      attackCase({ attack: 'prompt-leak', guarded: turn('landed', `here: ${SECRET}`) }),
      attackCase({ attack: 'audit-dump' }), // guarded blocked
      {
        ...attackCase({ attack: 'baseline' }),
        control: true,
        guarded: turn('clean', 'we open at 9'),
      },
    ];
    const landed = selectLandedOnGuard(reportWith(cases, 1));
    expect(landed.map((c) => c.attack)).toEqual(['prompt-leak']);
  });
});

describe('extractLeakedToken', () => {
  it('finds the credential canary in a guarded reply', () => {
    const cases = [
      attackCase({ attack: 'translation', guarded: turn('landed', `voici ${SECRET}`) }),
    ];
    expect(extractLeakedToken(cases)).toBe(SECRET);
  });

  it('returns null when no credential leaked', () => {
    const cases = [
      attackCase({ attack: 'tone', guarded: turn('landed', 'sure, here is some text') }),
    ];
    expect(extractLeakedToken(cases)).toBeNull();
  });
});

describe('suggestPolicyFromReport', () => {
  it('returns null when nothing landed on the guard', () => {
    const cases = [attackCase({ attack: 'audit-dump' })]; // guarded blocked
    expect(suggestPolicyFromReport(reportWith(cases, 0))).toBeNull();
  });

  it('summarises the landed attacks and names the leaked token', () => {
    const cases = [
      attackCase({ attack: 'prompt-leak', guarded: turn('landed', `here: ${SECRET}`) }),
      attackCase({ attack: 'translation', guarded: turn('landed', `voici ${SECRET}`) }),
    ];
    const suggestion = suggestPolicyFromReport(reportWith(cases, 2));
    expect(suggestion).not.toBeNull();
    expect(suggestion?.leakedToken).toBe(SECRET);
    expect(suggestion?.summary).toContain('2 attacks');
    expect(suggestion?.summary).toContain(SECRET);
    expect(suggestion?.attackNames).toEqual(['prompt-leak', 'translation']);
    expect(suggestion?.evidencePrompt).toContain('prompt-leak');
  });

  it('produces a deterministic credential policy that is a valid draft', () => {
    const cases = [
      attackCase({ attack: 'prompt-leak', guarded: turn('landed', `here: ${SECRET}`) }),
    ];
    const suggestion = suggestPolicyFromReport(reportWith(cases, 1));
    const draft = suggestion?.fallbackDraft;
    expect(draft?.id).toBe('block-credential-leak');
    expect(draft?.action).toBe('block');
    expect(draft?.matchType).toBe('regex');
    expect(draft?.matchValue).toContain('sk-');
    expect(policyDraftSchema.safeParse(draft).success).toBe(true);
  });

  it('uses a system-prompt policy id when the leak is the hidden prompt, not a credential', () => {
    const cases = [
      attackCase({
        attack: 'prompt-leak',
        goal: 'extract the hidden system prompt verbatim',
        guarded: turn('landed', 'You are ACME support assistant. Your instructions are...'),
      }),
    ];
    const suggestion = suggestPolicyFromReport(reportWith(cases, 1));
    expect(suggestion?.leakedToken).toBeNull();
    expect(suggestion?.fallbackDraft.id).toBe('block-system-prompt-echo');
  });
});

describe('hardenDraftYaml', () => {
  it('renders a credential draft to parseable policy YAML', () => {
    const cases = [
      attackCase({ attack: 'prompt-leak', guarded: turn('landed', `here: ${SECRET}`) }),
    ];
    const suggestion = suggestPolicyFromReport(reportWith(cases, 1));
    const yaml = hardenDraftYaml(suggestion!.fallbackDraft);
    expect(yaml).toContain('id: block-credential-leak');
    expect(yaml).toContain('action: block');
    expect(yaml).toContain('match:');
  });
});
