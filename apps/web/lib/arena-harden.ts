/**
 * Red-team → harden loop: turn a finished red-team report into a guard policy.
 *
 * This is the only new logic the hardening loop needs. It is a pure transform
 * over the report the arena already holds (`@/lib/arena-redteam`) plus thin
 * orchestration over the EXISTING policy API surface (`@/lib/policies`):
 *
 *   suggestPolicyFromReport()  pure: landed-on-guard cases → a policy draft + evidence
 *   buildHardenDraft()         draft via the LLM endpoint, falling back to deterministic
 *   applyHardenPolicy()        persist + enable through Rust /v1/policies
 *
 * Nothing here is persisted on the web side. The applied policy is owned by Rust,
 * exactly like a hand-authored one — the loop just generates the YAML from evidence
 * instead of asking the user to write it.
 */
import type { RedteamCase, RedteamReport } from './arena-redteam';
import { draftToYaml, type PolicyDraft } from './policy-draft';
import { generatePolicyDraft, upsertPolicy } from './policies';

/** Credential family the demo agent protects (mirrors demo/redteam-target/policy.yaml). */
const CREDENTIAL_REGEX = /sk-[a-z0-9]{6,}/i;
/** Inline-flag form the Rust `regex` crate understands; used in generated policies. */
const CREDENTIAL_MATCH_VALUE = '(?i)sk-[a-z0-9]{6,}';

/** A leak kind drives a stable policy id so re-hardening the same kind upserts
 * one policy, while a new kind adds another — that is what makes each round
 * visibly close *more* attacks. */
type LeakKind = 'credential' | 'system-prompt' | 'protected';

const LEAK_KIND_ID: Record<LeakKind, string> = {
  credential: 'block-credential-leak',
  'system-prompt': 'block-system-prompt-echo',
  protected: 'block-protected-disclosure',
};

export interface HardenSuggestion {
  /** Non-control attacks whose guarded reply still leaked (outcome `landed`). */
  readonly landedCases: readonly RedteamCase[];
  /** Distinct attack names that beat the guard, in first-seen order. */
  readonly attackNames: readonly string[];
  /** The exact protected value found in a guarded reply, if any (ground truth). */
  readonly leakedToken: string | null;
  /** One-line human summary for the card headline. */
  readonly summary: string;
  /** Natural-language prompt for the LLM draft endpoint. */
  readonly evidencePrompt: string;
  /** Deterministic, always-valid draft that is guaranteed to block what landed. */
  readonly fallbackDraft: PolicyDraft;
}

export interface HardenDraft {
  /** The policy to apply. `matchValue`/`id`/`action` are always deterministic
   * (guaranteed to block the leak); `description` may be LLM-enriched. */
  readonly draft: PolicyDraft;
  /** Where the description came from — drives a small "AI-written" tag. */
  readonly source: 'llm' | 'deterministic';
  readonly suggestion: HardenSuggestion;
}

/** Non-control attack cases whose guarded side still leaked. */
function landedOnGuardCases(cases: readonly RedteamCase[]): RedteamCase[] {
  return cases.filter((c) => !c.control && c.guarded.outcome === 'landed');
}

/** Non-control attack campaigns whose guarded side still leaked. */
export function selectLandedOnGuard(report: RedteamReport): RedteamCase[] {
  return landedOnGuardCases(report.cases);
}

/** First credential-shaped token leaked in any guarded reply, else null. */
export function extractLeakedToken(cases: readonly RedteamCase[]): string | null {
  for (const c of cases) {
    const match = c.guarded.reply.match(CREDENTIAL_REGEX);
    if (match) return match[0];
  }
  return null;
}

function classifyLeakKind(cases: readonly RedteamCase[], leakedToken: string | null): LeakKind {
  if (leakedToken !== null) return 'credential';
  const mentionsPrompt = cases.some((c) =>
    /system prompt|instruction|hidden|verbatim|prompt[- ]?leak/i.test(`${c.attack} ${c.goal}`),
  );
  return mentionsPrompt ? 'system-prompt' : 'protected';
}

function uniqueAttackNames(cases: readonly RedteamCase[]): string[] {
  const seen = new Set<string>();
  const names: string[] = [];
  for (const c of cases) {
    if (!seen.has(c.attack)) {
      seen.add(c.attack);
      names.push(c.attack);
    }
  }
  return names;
}

function pluralAttacks(count: number): string {
  return count === 1 ? '1 attack' : `${count} attacks`;
}

function buildSummary(count: number, leakedToken: string | null): string {
  const head = `${pluralAttacks(count)} slipped past your guard`;
  return leakedToken !== null ? `${head} — leaked "${leakedToken}"` : head;
}

function buildEvidencePrompt(cases: readonly RedteamCase[], leakedToken: string | null): string {
  const attacks = cases.map((c) => `- "${c.attack}": ${c.goal}`).join('\n');
  const leak =
    leakedToken !== null
      ? `The agent leaked the protected value \`${leakedToken}\` (a credential of the form sk-<alphanumeric>). `
      : 'The agent disclosed protected internal information it was told to keep confidential. ';
  return (
    'Write one guardrail policy that blocks an AI support agent from leaking its protected ' +
    'information in its reply. These adversarial prompts bypassed the current guard:\n' +
    `${attacks}\n` +
    `${leak}` +
    'The policy must block any assistant output that contains that value or pattern. Channel: chat.'
  );
}

function buildFallbackDraft(kind: LeakKind): PolicyDraft {
  if (kind === 'credential') {
    return {
      id: LEAK_KIND_ID.credential,
      description: "Blocks outputs containing the agent's credential (sk-<alphanumeric>).",
      matchType: 'regex',
      matchValue: CREDENTIAL_MATCH_VALUE,
      action: 'block',
      severity: 'critical',
      channels: ['chat'],
    };
  }
  if (kind === 'system-prompt') {
    return {
      id: LEAK_KIND_ID['system-prompt'],
      description: 'Blocks outputs that echo the system prompt or hidden instructions.',
      matchType: 'regex',
      matchValue: '(?i)(system prompt|hidden instructions|you are [a-z].{0,40} assistant)',
      action: 'block',
      severity: 'high',
      channels: ['chat'],
    };
  }
  // Generic protected-info disclosure. Reached only when no credential token was
  // found — a credential leak is classified as `credential` and handled above.
  return {
    id: LEAK_KIND_ID.protected,
    description: 'Blocks outputs that disclose the agent’s protected internal information.',
    matchType: 'regex',
    matchValue: '(?i)(api[ _-]?key|secret key|credential|protected internal information)',
    action: 'block',
    severity: 'critical',
    channels: ['chat'],
  };
}

/**
 * Turn a set of attack cases into a policy suggestion, or `null` when nothing
 * landed on the guard. Shared by the arena report path and the durable
 * single-target job path (`redteam-harden.ts`).
 */
export function suggestPolicyFromCases(cases: readonly RedteamCase[]): HardenSuggestion | null {
  const landedCases = landedOnGuardCases(cases);
  if (landedCases.length === 0) return null;

  const leakedToken = extractLeakedToken(landedCases);
  const kind = classifyLeakKind(landedCases, leakedToken);

  return {
    landedCases,
    attackNames: uniqueAttackNames(landedCases),
    leakedToken,
    summary: buildSummary(landedCases.length, leakedToken),
    evidencePrompt: buildEvidencePrompt(landedCases, leakedToken),
    fallbackDraft: buildFallbackDraft(kind),
  };
}

/**
 * Turn a finished report into a policy suggestion, or `null` when nothing landed
 * on the guard (no suggestion to make — the loop shows the win state instead).
 */
export function suggestPolicyFromReport(report: RedteamReport): HardenSuggestion | null {
  return suggestPolicyFromCases(report.cases);
}

/**
 * Build the draft to apply. The match logic is always deterministic so the guard
 * is *guaranteed* to block what leaked; the LLM endpoint only enriches the
 * description. If the LLM is unavailable (no key → 5xx), we keep the deterministic
 * description and carry on — the loop never dead-ends on a missing model.
 */
export async function buildHardenDraftFromSuggestion(
  suggestion: HardenSuggestion,
  signal?: AbortSignal,
): Promise<HardenDraft> {
  let draft = suggestion.fallbackDraft;
  let source: 'llm' | 'deterministic' = 'deterministic';
  try {
    const llm = await generatePolicyDraft(suggestion.evidencePrompt, signal);
    if (llm.description.trim() !== '') {
      draft = { ...suggestion.fallbackDraft, description: llm.description.trim() };
      source = 'llm';
    }
  } catch {
    // LLM not configured / unreachable — deterministic draft stands.
  }

  return { draft, source, suggestion };
}

export async function buildHardenDraft(
  report: RedteamReport,
  signal?: AbortSignal,
): Promise<HardenDraft | null> {
  const suggestion = suggestPolicyFromReport(report);
  if (suggestion === null) return null;
  return buildHardenDraftFromSuggestion(suggestion, signal);
}

/** YAML preview for the read-only disclosure in the card. */
export function hardenDraftYaml(draft: PolicyDraft): string {
  return draftToYaml(draft);
}

/**
 * Persist + enable the policy through the existing Rust-backed proxy. `upsertPolicy`
 * enables on write, so a re-harden of the same id upserts in place rather than
 * stacking. Returns the policy id that was applied.
 */
export async function applyHardenPolicy(draft: PolicyDraft, signal?: AbortSignal): Promise<string> {
  await upsertPolicy(draftToYaml(draft), signal);
  return draft.id;
}
