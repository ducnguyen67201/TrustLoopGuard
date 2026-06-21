/**
 * Same-origin client for the durable red-team job API.
 *
 * The browser calls the authed routes under `/api/redteam/*`, which proxy to the
 * Rust orchestrator (`crates/tl-server/src/redteam`); Rust owns the durable job +
 * per-attack results. The dashboard only dispatches, polls, lists, and cancels.
 *
 * The dashboard cannot use `@trustloopguard/sdk`'s `Client` here: that targets
 * Rust `/v1/*` directly with a bearer key (customer runtime), whereas the browser
 * authenticates by session through this same-origin proxy. So this is a thin
 * client over `/api/redteam/*` — but its TYPES are single-sourced from Rust via
 * the generated SDK bindings, and zod validates every response at the boundary.
 */
import type {
  JobStatus,
  RedteamJobDetail,
  RedteamJobResult,
  RedteamJobSummary,
  RedteamReportShare,
} from '@trustloopguard/sdk';
// Vectors enter the browser via the zod-validated planner client; reuse its
// inferred type so the optional `source_path` shape lines up at this boundary.
import type { AttackVector } from './redteam-plan';
import { z } from 'zod';

export type {
  JobStatus,
  RedteamJobDetail,
  RedteamJobResult,
  RedteamJobSummary,
  RedteamReportShare,
};

// Web-only: the dashboard offers exactly these profiles. `profile` is a free
// String on the wire, so this enum is a UI constraint, not a generated wire type.
export const REDTEAM_JOB_PROFILES = ['fast', 'full', 'max'] as const;
export const redteamJobProfileSchema = z.enum(REDTEAM_JOB_PROFILES);
export type RedteamJobProfile = z.infer<typeof redteamJobProfileSchema>;

export const REDTEAM_RUN_MODES = ['one_off', 'learning'] as const;
export const redteamRunModeSchema = z.enum(REDTEAM_RUN_MODES);
export type RedteamRunMode = z.infer<typeof redteamRunModeSchema>;

export const REDTEAM_ATTACK_SURFACES = ['chat', 'document_workflow'] as const;
export const redteamAttackSurfaceSchema = z.enum(REDTEAM_ATTACK_SURFACES);
export type RedteamAttackSurface = z.infer<typeof redteamAttackSurfaceSchema>;

/** Terminal states stop polling. */
export function isTerminalStatus(status: JobStatus): boolean {
  return status === 'complete' || status === 'error' || status === 'cancelled';
}

/** Share of non-control attacks that landed, as a 0–100 integer percentage. */
export function landedPercent(job: RedteamJobSummary): number {
  return job.attacks > 0 ? Math.round((job.landed / job.attacks) * 100) : 0;
}

const jobStatusSchema = z.enum(['queued', 'running', 'complete', 'error', 'cancelled']);

export const redteamJobSummarySchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  environment_id: z.string(),
  status: jobStatusSchema,
  target: z.string(),
  // Dispatch validates profile ∈ {fast,full,max} server-side; constrain here too
  // to catch backend drift (narrower than the wire's String, still assignable).
  profile: redteamJobProfileSchema,
  // Wire sends `null` (not an omitted key) for absent optionals — see redteam.rs.
  agent_id: z.string().nullable(),
  attacks: z.number(),
  landed: z.number(),
  blocked: z.number(),
  error: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

const redteamJobResultSchema = z
  .object({
    seq: z.number(),
    // Optional benchmark metadata already exists in Rust. Omitted when absent.
    case_id: z.string().optional(),
    track: z.string().optional(),
    kind: z.string().optional(),
    trial_index: z.number().optional(),
    attack: z.string(),
    goal: z.string(),
    outcome: z.string(),
    landed: z.boolean(),
    prompt: z.string().nullable(),
    reply: z.string(),
    trace_id: z.string().nullable(),
  })
  .transform((result): RedteamJobResult => {
    const parsed: RedteamJobResult = {
      seq: result.seq,
      attack: result.attack,
      goal: result.goal,
      outcome: result.outcome,
      landed: result.landed,
      prompt: result.prompt,
      reply: result.reply,
      trace_id: result.trace_id,
    };
    if (result.case_id !== undefined) parsed.case_id = result.case_id;
    if (result.track !== undefined) parsed.track = result.track;
    if (result.kind !== undefined) parsed.kind = result.kind;
    if (result.trial_index !== undefined) parsed.trial_index = result.trial_index;
    return parsed;
  });

export const redteamJobDetailSchema = z.object({
  job: redteamJobSummarySchema,
  results: z.array(redteamJobResultSchema),
});

const redteamJobListResponseSchema = z.object({
  jobs: z.array(redteamJobSummarySchema),
});

const redteamReportShareSchema = z.object({
  token: z.string(),
  path: z.string(),
  job_id: z.string(),
  compare_job_id: z.string().nullable(),
  created_at: z.string(),
  expires_at: z.string().nullable(),
});

export interface CreateReportInput {
  jobId: string;
  compareJobId?: string;
  ttlDays?: number;
}

interface DispatchInput {
  targetUrl: string;
  profile: RedteamJobProfile;
  mode?: RedteamRunMode;
  attackSurface?: RedteamAttackSurface;
  agentId?: string;
  documentTemplate?: DocumentTemplateInput;
  /** Tailored seeds from `redteam/plan`; already in the Rust wire shape. */
  attackVectors?: AttackVector[];
}

export interface DocumentTemplateInput {
  fileName: string;
  mediaType: string;
  dataBase64: string;
  fields?: Record<string, string>;
  flatten?: boolean;
}

type DocumentTemplateWire = {
  file_name: string;
  media_type: string;
  data_base64: string;
  fields?: Record<string, string>;
  flatten: boolean;
};

type DispatchBody = {
  target_url: string;
  profile: RedteamJobProfile;
  mode: RedteamRunMode;
  attack_surface: RedteamAttackSurface;
  agent_id?: string;
  document_template?: DocumentTemplateWire;
  attack_vectors?: AttackVector[];
};

export interface ListJobsParams {
  agentId?: string;
  limit?: number;
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

/** One same-origin call to `/api/redteam{path}`: fetch → surface errors → validate. */
async function request<S extends z.ZodTypeAny>(
  path: string,
  schema: S,
  init?: RequestInit,
): Promise<z.infer<S>> {
  const response = await fetch(`/api/redteam${path}`, init);
  const json = await readJson(response);
  if (!response.ok) throw new Error(messageFromBody(json, response.status));
  return schema.parse(json);
}

/** Translate the UI's camelCase dispatch shape to the Rust wire contract (snake_case). */
function dispatchBody(input: DispatchInput): DispatchBody {
  const body: DispatchBody = {
    target_url: input.targetUrl,
    profile: input.profile,
    mode: input.mode ?? 'one_off',
    attack_surface: input.attackSurface ?? 'chat',
  };
  if (input.agentId !== undefined && input.agentId !== '') body.agent_id = input.agentId;
  if (input.documentTemplate !== undefined) {
    const documentTemplate: DocumentTemplateWire = {
      file_name: input.documentTemplate.fileName,
      media_type: input.documentTemplate.mediaType,
      data_base64: input.documentTemplate.dataBase64,
      flatten: input.documentTemplate.flatten ?? false,
    };
    if (input.documentTemplate.fields !== undefined) {
      documentTemplate.fields = input.documentTemplate.fields;
    }
    body.document_template = documentTemplate;
  }
  // Vectors are already in the Rust wire shape (snake_case AttackVector) — no
  // per-field translation needed.
  if (input.attackVectors !== undefined && input.attackVectors.length > 0) {
    body.attack_vectors = input.attackVectors;
  }
  return body;
}

function jobsQuery(params?: ListJobsParams): string {
  const query = new URLSearchParams();
  if (params?.agentId !== undefined && params.agentId !== '') query.set('agent_id', params.agentId);
  if (params?.limit !== undefined) query.set('limit', String(params.limit));
  const serialized = query.toString();
  return serialized === '' ? '' : `?${serialized}`;
}

/** Translate the UI's camelCase report shape to the Rust wire contract (snake_case). */
function createReportBody(input: CreateReportInput): {
  job_id: string;
  compare_job_id?: string;
  ttl_days?: number;
} {
  const body: { job_id: string; compare_job_id?: string; ttl_days?: number } = {
    job_id: input.jobId,
  };
  if (input.compareJobId !== undefined && input.compareJobId !== '') {
    body.compare_job_id = input.compareJobId;
  }
  if (input.ttlDays !== undefined) body.ttl_days = input.ttlDays;
  return body;
}

/** Durable red-team job client (same-origin `/api/redteam/*` proxy to Rust). */
export const redteam = {
  /** Dispatch a job. Returns the persisted `Queued` summary with its id. */
  dispatch(input: DispatchInput): Promise<RedteamJobSummary> {
    return request('/dispatch', redteamJobSummarySchema, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(dispatchBody(input)),
    });
  },

  /** Fetch a job and its per-attack results. */
  getJob(id: string): Promise<RedteamJobDetail> {
    return request(`/jobs/${encodeURIComponent(id)}`, redteamJobDetailSchema);
  },

  /** List recent jobs in the workspace, newest first. */
  async listJobs(params?: ListJobsParams): Promise<RedteamJobSummary[]> {
    return (await request(`/jobs${jobsQuery(params)}`, redteamJobListResponseSchema)).jobs;
  },

  /** Cooperatively cancel a job; returns the updated (or unchanged terminal) summary. */
  cancel(id: string): Promise<RedteamJobSummary> {
    return request(`/jobs/${encodeURIComponent(id)}/cancel`, redteamJobSummarySchema, {
      method: 'POST',
    });
  },

  /** Mint a shareable report link for a completed job (optionally a same-agent comparison). */
  createReport(input: CreateReportInput): Promise<RedteamReportShare> {
    return request('/reports', redteamReportShareSchema, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(createReportBody(input)),
    });
  },

  /** Revoke a shareable report link. The public PDF then 404s. */
  async revokeReport(token: string): Promise<void> {
    const response = await fetch(`/api/redteam/reports/${encodeURIComponent(token)}/revoke`, {
      method: 'POST',
    });
    if (!response.ok) throw new Error(messageFromBody(await readJson(response), response.status));
  },
};
