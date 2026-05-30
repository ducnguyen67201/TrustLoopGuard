import { z } from 'zod';

import type { RunRow } from '@/lib/server/dashboard-data';
import {
  formatDateTime,
  metadataEntries,
  relativeTime,
  shortRunId,
  titleize,
} from './run-format';

const runSummarySchema = z.object({
  id: z.string(),
  environment_id: z.string(),
  environment: z.string(),
  agent_id: z.string(),
  kind: z.string(),
  status: z.string(),
  external_id: z.string().nullable(),
  metadata: z.object({}).passthrough(),
  started_at: z.string(),
  ended_at: z.string().nullable(),
  trace_count: z.number(),
  blocked_count: z.number(),
  rewritten_count: z.number(),
  escalated_count: z.number(),
  p95_latency_ms: z.number().nullable(),
});

export const runListWireSchema = z.object({
  runs: z.array(runSummarySchema),
});

export type RunListWire = z.infer<typeof runListWireSchema>;

export function parseRunsSnapshot(
  value: Awaited<ReturnType<Response['json']>>,
  workspaceSlug: string,
): RunRow[] {
  const parsed = runListWireSchema.safeParse(value);
  if (!parsed.success) {
    throw new Error('runs list contract validation failed');
  }

  return parsed.data.runs.map((run) => runRow(run, workspaceSlug));
}

function runRow(run: RunListWire['runs'][number], workspaceSlug: string): RunRow {
  return {
    id: run.id,
    shortId: shortRunId(run.id),
    agent: run.agent_id,
    environment: run.environment,
    kind: titleize(run.kind),
    status: titleize(run.status),
    externalId: run.external_id?.trim() || 'None',
    traces: run.trace_count,
    blocked: run.blocked_count,
    rewritten: run.rewritten_count,
    escalated: run.escalated_count,
    p95LatencyMs: run.p95_latency_ms,
    latency: run.p95_latency_ms === null ? 'No traces' : `${run.p95_latency_ms}ms`,
    started: relativeTime(new Date(run.started_at)),
    startedAt: formatDateTime(new Date(run.started_at)),
    endedAt: run.ended_at ? formatDateTime(new Date(run.ended_at)) : 'Still running',
    metadata: metadataEntries(run.metadata),
    href: `/runs/${encodeURIComponent(run.id)}?workspace=${encodeURIComponent(workspaceSlug)}&environment=${encodeURIComponent(run.environment_id)}`,
  };
}
