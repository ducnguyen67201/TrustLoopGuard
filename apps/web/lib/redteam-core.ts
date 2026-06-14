/**
 * Shared red-team wire contract for the web side.
 *
 * These zod schemas are the single source of truth for the red-team report shape
 * the dashboard consumes; the runner emits the identical JSON. The durable Attacks
 * path (`/api/redteam/*` → Rust `/v1/redteam/*`) and the harden loop
 * (`redteam-harden.ts`) builds on these types.
 *
 * The runner, the attack engine, and the LLMs it drives are intentionally not named
 * here — this layer only speaks "red-team".
 */
import { z } from 'zod';

const REDTEAM_PROFILES = ['fast', 'full', 'max'] as const;
const redteamProfileSchema = z.enum(REDTEAM_PROFILES);

const redteamRunStatusSchema = z.enum(['running', 'complete', 'error']);

/** Per-target outcome for one attack campaign. */
const redteamOutcomeSchema = z.enum(['landed', 'blocked', 'clean', 'error']);

const redteamTurnSchema = z.object({
  outcome: redteamOutcomeSchema,
  reply: z.string(),
  detail: z.string(),
  traceId: z.string().nullable(),
});
export const redteamCaseSchema = z.object({
  attack: z.string(),
  goal: z.string(),
  /** A control case (clean traffic) is excluded from the success-rate denominator. */
  control: z.boolean(),
  prompt: z.string().nullable(),
  raw: redteamTurnSchema,
  guarded: redteamTurnSchema,
});
export type RedteamCase = z.infer<typeof redteamCaseSchema>;

export const redteamTargetSummarySchema = z.object({
  total: z.number(),
  /** Non-control attack campaigns — the success-rate denominator. */
  attacks: z.number(),
  landed: z.number(),
  blocked: z.number(),
  clean: z.number(),
  errored: z.number(),
  /** landed / attacks, in [0, 1]. */
  successRate: z.number(),
});
export type RedteamTargetSummary = z.infer<typeof redteamTargetSummarySchema>;

const redteamLlmSchema = z.object({
  mode: z.string(),
  generator: z.string(),
  judge: z.string(),
});
export const redteamReportSchema = z.object({
  profile: redteamProfileSchema,
  status: redteamRunStatusSchema,
  llm: redteamLlmSchema,
  raw: redteamTargetSummarySchema,
  guarded: redteamTargetSummarySchema,
  /** Percentage-point drop in success rate, raw minus guarded. */
  deltaPoints: z.number(),
  cases: z.array(redteamCaseSchema),
  progress: z.object({ done: z.number(), total: z.number() }),
  error: z.string().nullable(),
});
export type RedteamReport = z.infer<typeof redteamReportSchema>;

// SSRF guard. Agent target URLs are user-controlled and are ultimately fetched
// server-side by the red-team runner. Red-team only ever targets loopback agents,
// so we ALLOW loopback and reject everything else (cloud metadata 169.254.x.x,
// internal hosts, external exfil). An allowlist is deny-by-default and immune to the
// DNS-rebinding gap a loopback *denylist* would have.
const ALLOWED_AGENT_HOSTS = new Set(['127.0.0.1', 'localhost', '::1']);

/** True only for http(s) URLs pointing at a loopback agent. */
export function isAllowedAgentTargetUrl(raw: string): boolean {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return false;
  const host = url.hostname.replace(/^\[|\]$/g, '').toLowerCase();
  return ALLOWED_AGENT_HOSTS.has(host);
}
