/**
 * Types + zod schemas for the red-team vulnerability report, single-sourced from
 * the generated SDK (`@trustloopguard/sdk`). The public PDF route uses these to
 * fetch and validate the Rust-owned report payload before rendering.
 *
 * `fetchPublicReport` is server-only: it reads `env.TL_SERVER_URL`. The
 * `server-only` guard makes the bundler error if this module is ever imported
 * into a client component (type-only imports stay safe — they are erased).
 */
import 'server-only';

import type {
  ComparedAttackStatus,
  RedteamComparedAttack,
  RedteamReportAggregates,
  RedteamReportComparison,
  RedteamReportFinding,
  RedteamReportPayload,
  ReportSeverity,
} from '@trustloopguard/sdk';
import { z } from 'zod';

import { env } from '@/env';

import { redteamJobSummarySchema } from './redteam-jobs';

export type {
  ComparedAttackStatus,
  RedteamComparedAttack,
  RedteamReportAggregates,
  RedteamReportComparison,
  RedteamReportFinding,
  RedteamReportPayload,
  ReportSeverity,
};

export const reportSeveritySchema = z.enum(['critical', 'high', 'medium', 'low', 'info']);
export const comparedAttackStatusSchema = z.enum([
  'fixed',
  'still_vulnerable',
  'regressed',
  'unchanged',
]);

export const redteamReportFindingSchema = z.object({
  seq: z.number(),
  attack: z.string(),
  goal: z.string(),
  category: z.string(),
  severity: reportSeveritySchema,
  outcome: z.string(),
  landed: z.boolean(),
  evidence: z.string().nullable(),
  prompt: z.string().nullable(),
  trace_id: z.string().nullable(),
});

export const redteamReportAggregatesSchema = z.object({
  total: z.number(),
  attacks: z.number(),
  landed: z.number(),
  blocked: z.number(),
  clean: z.number(),
  errored: z.number(),
  success_rate: z.number(),
  risk_level: reportSeveritySchema,
});

export const redteamComparedAttackSchema = z.object({
  attack: z.string(),
  goal: z.string(),
  baseline_outcome: z.string(),
  compare_outcome: z.string().nullable(),
  status: comparedAttackStatusSchema,
});

export const redteamReportComparisonSchema = z.object({
  baseline: redteamJobSummarySchema,
  compare: redteamJobSummarySchema,
  baseline_aggregates: redteamReportAggregatesSchema,
  compare_aggregates: redteamReportAggregatesSchema,
  delta_points: z.number(),
  attacks: z.array(redteamComparedAttackSchema),
});

export const redteamReportPayloadSchema = z.object({
  job: redteamJobSummarySchema,
  aggregates: redteamReportAggregatesSchema,
  findings: z.array(redteamReportFindingSchema),
  comparison: redteamReportComparisonSchema.nullable(),
  generated_at: z.string(),
});

/** Thrown when a token is unknown, expired, or revoked (Rust returns 404). */
export class ReportNotFoundError extends Error {
  constructor() {
    super('report not found or expired');
    this.name = 'ReportNotFoundError';
  }
}

/**
 * Fetch the report payload for a share token straight from the Rust public
 * endpoint. No workspace auth — the token is the capability. Server-only.
 */
export async function fetchPublicReport(token: string): Promise<RedteamReportPayload> {
  const base = env.TL_SERVER_URL.replace(/\/$/, '');
  const response = await fetch(`${base}/v1/redteam/reports/${encodeURIComponent(token)}`, {
    // The report reflects current job data; never cache a bearer-scoped fetch.
    cache: 'no-store',
  });
  if (response.status === 404) throw new ReportNotFoundError();
  if (!response.ok) throw new Error(`report fetch failed (HTTP ${response.status})`);
  const json: unknown = await response.json();
  return redteamReportPayloadSchema.parse(json);
}
