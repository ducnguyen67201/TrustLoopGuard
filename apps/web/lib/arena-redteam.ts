/**
 * Shared contract for the arena red-team flow.
 *
 * The browser calls the same-origin Next route `/api/arena/redteam`, which proxies
 * to a standalone red-team backend. These zod schemas are the single source of truth
 * for the report shape on the web side; the backend emits the identical JSON.
 *
 * The backend, the attack engine, and the LLMs it drives are intentionally not named
 * here — this layer only speaks "red-team".
 */
import { z } from 'zod';

export const REDTEAM_PROFILES = ['fast', 'full', 'max'] as const;
export const redteamProfileSchema = z.enum(REDTEAM_PROFILES);
export type RedteamProfile = z.infer<typeof redteamProfileSchema>;

export const redteamRunRequestSchema = z.object({
  profile: redteamProfileSchema,
  rawUrl: z.string().url().optional(),
  guardedUrl: z.string().url().optional(),
});
export type RedteamRunRequest = z.infer<typeof redteamRunRequestSchema>;

export const redteamRunStatusSchema = z.enum(['running', 'complete', 'error']);
export type RedteamRunStatus = z.infer<typeof redteamRunStatusSchema>;

/** Per-target outcome for one attack campaign. */
export const redteamOutcomeSchema = z.enum(['landed', 'blocked', 'clean', 'error']);
export type RedteamOutcome = z.infer<typeof redteamOutcomeSchema>;

export const redteamTurnSchema = z.object({
  outcome: redteamOutcomeSchema,
  reply: z.string(),
  detail: z.string(),
  traceId: z.string().nullable(),
});
export type RedteamTurn = z.infer<typeof redteamTurnSchema>;

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

export const redteamLlmSchema = z.object({
  mode: z.string(),
  generator: z.string(),
  judge: z.string(),
});
export type RedteamLlm = z.infer<typeof redteamLlmSchema>;

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

export const redteamRunHandleSchema = z.object({
  runId: z.string(),
  status: redteamRunStatusSchema,
});
export type RedteamRunHandle = z.infer<typeof redteamRunHandleSchema>;

export const redteamRunPollSchema = z.object({
  runId: z.string(),
  status: redteamRunStatusSchema,
  report: redteamReportSchema.nullable(),
});
export type RedteamRunPoll = z.infer<typeof redteamRunPollSchema>;

const errorEnvelopeSchema = z.object({ error: z.string() });

/** 0-100 integer for display. */
export function successPercent(summary: RedteamTargetSummary): number {
  return Math.round(summary.successRate * 100);
}

function messageFromBody(body: unknown, status: number): string {
  const parsed = errorEnvelopeSchema.safeParse(body);
  if (parsed.success) return parsed.data.error;
  return `red-team request failed (HTTP ${status})`;
}

async function readJson(response: Response): Promise<unknown> {
  const text = await response.text();
  if (text === '') return {};
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return {};
  }
}

/** Kick off a run via the same-origin Next proxy. */
export async function startRedteamRun(request: RedteamRunRequest): Promise<RedteamRunHandle> {
  const response = await fetch('/api/arena/redteam', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(request),
  });
  const body = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(body, response.status));
  return redteamRunHandleSchema.parse(body);
}

/** Poll a run's status + latest report via the same-origin Next proxy. */
export async function pollRedteamRun(runId: string): Promise<RedteamRunPoll> {
  const response = await fetch(`/api/arena/redteam?runId=${encodeURIComponent(runId)}`);
  const body = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(body, response.status));
  return redteamRunPollSchema.parse(body);
}
