/**
 * Client contract for the Attacks tab: attack a SINGLE agent by URL.
 *
 * Unlike the arena (raw-vs-guarded pair), the dashboard Attacks tab points an
 * independent red-team at one agent endpoint and reports what got through. The
 * browser calls the same-origin authed route `/api/attacks`, which proxies to the
 * standalone runner in single-target mode. The report shape is shared with the
 * arena (`@/lib/arena-redteam`): in single-target mode the runner fills `guarded`
 * with the target's results and zeroes `raw`.
 */
import { z } from 'zod';

import {
  redteamProfileSchema,
  redteamRunHandleSchema,
  redteamRunPollSchema,
  type RedteamProfile,
  type RedteamReport,
  type RedteamRunHandle,
  type RedteamRunPoll,
} from './arena-redteam';

export const attackRunRequestSchema = z.object({
  profile: redteamProfileSchema,
  targetUrl: z.string().url(),
});
export type AttackRunRequest = z.infer<typeof attackRunRequestSchema>;

export type { RedteamProfile, RedteamReport, RedteamRunHandle, RedteamRunPoll };

const errorEnvelopeSchema = z.object({ error: z.string() });

function messageFromBody(body: unknown, status: number): string {
  const parsed = errorEnvelopeSchema.safeParse(body);
  if (parsed.success) return parsed.data.error;
  return `attack request failed (HTTP ${status})`;
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

/** Kick off a single-target attack via the same-origin authed proxy. */
export async function startAttackRun(request: AttackRunRequest): Promise<RedteamRunHandle> {
  const response = await fetch('/api/attacks', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(request),
  });
  const body = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(body, response.status));
  return redteamRunHandleSchema.parse(body);
}

/** Poll a run's status + latest report via the same-origin authed proxy. */
export async function pollAttackRun(runId: string): Promise<RedteamRunPoll> {
  const response = await fetch(`/api/attacks?runId=${encodeURIComponent(runId)}`);
  const body = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(body, response.status));
  return redteamRunPollSchema.parse(body);
}
