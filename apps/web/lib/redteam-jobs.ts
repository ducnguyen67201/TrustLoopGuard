/**
 * Client for the durable red-team job API.
 *
 * The browser calls the same-origin authed routes under `/api/redteam/*`, which
 * proxy to the Rust orchestrator (`crates/tl-server/src/redteam`); Rust owns the
 * durable job + per-attack results. The dashboard only dispatches, polls, lists,
 * and cancels.
 *
 * TYPES are single-sourced from Rust via the generated SDK bindings
 * (`@trustloopguard/sdk`, produced by `cargo run -p tl-codegen`). The zod schemas
 * below are the runtime validators for the fetch boundary (the dashboard parses
 * untrusted HTTP); each function returns the generated type, so a wire change the
 * schemas fail to mirror surfaces as a compile error here.
 */
import type {
  JobStatus,
  RedteamGenerator,
  RedteamJobDetail,
  RedteamJobResult,
  RedteamJobSummary,
} from '@trustloopguard/sdk';
import { z } from 'zod';

export type { JobStatus, RedteamGenerator, RedteamJobDetail, RedteamJobResult, RedteamJobSummary };

// Web-only: the dashboard offers exactly these profiles. `profile` is a free
// String on the wire, so this enum is a UI constraint, not a generated wire type.
export const REDTEAM_JOB_PROFILES = ['fast', 'full', 'max'] as const;
export const redteamJobProfileSchema = z.enum(REDTEAM_JOB_PROFILES);
export type RedteamJobProfile = z.infer<typeof redteamJobProfileSchema>;

/** Terminal states stop polling. */
export function isTerminalStatus(status: JobStatus): boolean {
  return status === 'complete' || status === 'error' || status === 'cancelled';
}

export const jobStatusSchema = z.enum(['queued', 'running', 'complete', 'error', 'cancelled']);
export const redteamGeneratorSchema = z.enum(['deterministic', 'hackagent']);

export const redteamJobSummarySchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  environment_id: z.string(),
  status: jobStatusSchema,
  target: z.string(),
  // Dispatch validates profile ∈ {fast,full,max} server-side; constrain here too
  // to catch backend drift (narrower than the wire's String, still assignable).
  profile: redteamJobProfileSchema,
  generator: redteamGeneratorSchema,
  // Wire sends `null` (not an omitted key) for absent optionals — see redteam.rs.
  agent_id: z.string().nullable(),
  attacks: z.number(),
  landed: z.number(),
  blocked: z.number(),
  error: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const redteamJobResultSchema = z.object({
  seq: z.number(),
  attack: z.string(),
  goal: z.string(),
  outcome: z.string(),
  landed: z.boolean(),
  prompt: z.string().nullable(),
  reply: z.string(),
  trace_id: z.string().nullable(),
});

export const redteamJobDetailSchema = z.object({
  job: redteamJobSummarySchema,
  results: z.array(redteamJobResultSchema),
});

export const redteamJobListResponseSchema = z.object({
  jobs: z.array(redteamJobSummarySchema),
});

export interface DispatchInput {
  targetUrl: string;
  profile: RedteamJobProfile;
  generator?: RedteamGenerator;
  agentId?: string;
}

const errorEnvelopeSchema = z.object({ error: z.string() });

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

/** Dispatch a job. Returns the persisted `Queued` summary with its id. */
export async function dispatchJob(input: DispatchInput): Promise<RedteamJobSummary> {
  // Translate the UI's camelCase shape to the Rust wire contract (snake_case).
  const body: {
    target_url: string;
    profile: RedteamJobProfile;
    generator?: RedteamGenerator;
    agent_id?: string;
  } = {
    target_url: input.targetUrl,
    profile: input.profile,
  };
  if (input.generator !== undefined) body.generator = input.generator;
  if (input.agentId !== undefined && input.agentId !== '') body.agent_id = input.agentId;

  const response = await fetch('/api/redteam/dispatch', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return redteamJobSummarySchema.parse(json);
}

/** Fetch a job and its per-attack results. */
export async function getJob(id: string): Promise<RedteamJobDetail> {
  const response = await fetch(`/api/redteam/jobs/${encodeURIComponent(id)}`);
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return redteamJobDetailSchema.parse(json);
}

/** List recent jobs in the workspace, newest first. */
export async function listJobs(params?: {
  agentId?: string;
  limit?: number;
}): Promise<RedteamJobSummary[]> {
  const query = new URLSearchParams();
  if (params?.agentId !== undefined && params.agentId !== '') query.set('agent_id', params.agentId);
  if (params?.limit !== undefined) query.set('limit', String(params.limit));
  const suffix = query.toString() === '' ? '' : `?${query.toString()}`;
  const response = await fetch(`/api/redteam/jobs${suffix}`);
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return redteamJobListResponseSchema.parse(json).jobs;
}

/** Cooperatively cancel a job; returns the updated (or unchanged terminal) summary. */
export async function cancelJob(id: string): Promise<RedteamJobSummary> {
  const response = await fetch(`/api/redteam/jobs/${encodeURIComponent(id)}/cancel`, {
    method: 'POST',
  });
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return redteamJobSummarySchema.parse(json);
}
