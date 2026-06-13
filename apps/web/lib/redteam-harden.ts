/**
 * Red-team → harden loop for the durable single-target Attacks tab.
 *
 * The harden core (`harden-core.ts`) already turns landed-on-guard attacks into a
 * guard policy. A dispatched job carries the same evidence in a flatter shape
 * (`RedteamJobResult[]` instead of `RedteamReport.cases`), so this module just
 * adapts the results into cases and reuses the harden core — suggest, draft, apply.
 * The applied policy is owned by Rust (`/v1/policies`); re-running is a fresh
 * dispatch against the same target.
 */
import {
  applyHardenPolicy,
  buildHardenDraftFromSuggestion,
  hardenDraftYaml,
  suggestPolicyFromCases,
  type HardenDraft,
  type HardenSuggestion,
} from './harden-core';
import type { RedteamCase, RedteamOutcome } from './redteam-core';
import type { RedteamJobResult } from './redteam-jobs';

const KNOWN_OUTCOMES: ReadonlySet<RedteamOutcome> = new Set([
  'landed',
  'blocked',
  'clean',
  'error',
]);

function toOutcome(value: string): RedteamOutcome {
  return KNOWN_OUTCOMES.has(value as RedteamOutcome) ? (value as RedteamOutcome) : 'error';
}

/**
 * Adapt durable job results into arena `RedteamCase`s. Jobs already exclude
 * control probes, and the target's results map onto the `guarded` side (the
 * single target that was attacked); the `raw` side is left empty.
 */
export function jobResultsToCases(results: readonly RedteamJobResult[]): RedteamCase[] {
  return results.map((result) => ({
    attack: result.attack,
    goal: result.goal,
    control: false,
    prompt: result.prompt ?? null,
    raw: { outcome: 'clean', reply: '', detail: '', traceId: null },
    guarded: {
      outcome: toOutcome(result.outcome),
      reply: result.reply,
      detail: '',
      traceId: result.trace_id ?? null,
    },
  }));
}

/** Suggest a guard policy from a job's results, or `null` when nothing landed. */
export function suggestPolicyFromJobResults(
  results: readonly RedteamJobResult[],
): HardenSuggestion | null {
  return suggestPolicyFromCases(jobResultsToCases(results));
}

/** Build the draft to apply from a job's results (LLM-enriched, deterministic fallback). */
export async function buildHardenDraftFromJob(
  results: readonly RedteamJobResult[],
  signal?: AbortSignal,
): Promise<HardenDraft | null> {
  const suggestion = suggestPolicyFromJobResults(results);
  if (suggestion === null) return null;
  return buildHardenDraftFromSuggestion(suggestion, signal);
}

export { applyHardenPolicy, hardenDraftYaml };
export type { HardenDraft, HardenSuggestion };
