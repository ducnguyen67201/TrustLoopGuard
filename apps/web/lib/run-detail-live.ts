import { z } from 'zod';

const objectSchema = z.object({}).passthrough();

const triggeredPolicySchema = z
  .object({
    id: z.string().optional(),
    severity: z.string().optional(),
    reason: z.string().optional(),
  })
  .passthrough();

const runtimeDecisionPayloadSchema = z
  .object({
    trace_id: z.string().optional(),
    verdict: z.string().optional(),
    reason: z.string().optional(),
    triggered_policies: z.array(triggeredPolicySchema).optional(),
    safe_output: z.string().nullable().optional(),
    latency_ms: z.number().optional(),
    agent_id: z.string().optional(),
  })
  .passthrough();

const runSummarySchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  agent_id: z.string(),
  kind: z.string(),
  status: z.string(),
  external_id: z.string().nullable(),
  metadata: objectSchema,
  started_at: z.string(),
  ended_at: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
  trace_count: z.number(),
  blocked_count: z.number(),
  rewritten_count: z.number(),
  escalated_count: z.number(),
  p95_latency_ms: z.number().nullable(),
});

const runEventSummarySchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  run_id: z.string(),
  sequence: z.number(),
  kind: z.string(),
  label: z.string().nullable(),
  input_summary: z.string().nullable(),
  output_summary: z.string().nullable(),
  metadata: objectSchema,
  occurred_at: z.string(),
  created_at: z.string(),
});

const traceSummarySchema = z.object({
  trace_id: z.string(),
  run_id: z.string().nullable().optional(),
  run_event_id: z.string().nullable().optional(),
  domain: z.string(),
  decision: z.string(),
  elapsed_ms: z.number(),
  latest_review_outcome: z.string().nullable().optional(),
  latest_reviewed_at: z.string().nullable().optional(),
  payload: runtimeDecisionPayloadSchema,
  created_at: z.string(),
});

export const runDetailWireSchema = z.object({
  run: runSummarySchema,
  events: z.array(runEventSummarySchema),
  traces: z.array(traceSummarySchema),
});

type MetadataWire = z.infer<typeof objectSchema>;
type RuntimeDecisionPayloadWire = z.infer<typeof runtimeDecisionPayloadSchema>;
export type RunDetailWire = z.infer<typeof runDetailWireSchema>;

export type RunDetailSnapshot = {
  run: {
    id: string;
    shortId: string;
    agent: string;
    kind: string;
    status: string;
    externalId: string;
    traces: number;
    blocked: number;
    rewritten: number;
    escalated: number;
    latency: string;
    started: string;
    startedAt: string;
    endedAt: string;
    metadata: Array<{ label: string; value: string }>;
  };
  events: Array<{
    id: string;
    sequence: number;
    kind: string;
    label: string;
    input: string;
    output: string;
    time: string;
    metadata: Array<{ label: string; value: string }>;
  }>;
  traces: Array<{
    id: string;
    runEventId: string | null;
    phase: string;
    verdict: string;
    policy: string;
    reason: string;
    safeOutput: string | null;
    latency: string;
    time: string;
  }>;
};

export function parseRunDetailSnapshot(value: Awaited<ReturnType<Response['json']>>) {
  const parsed = runDetailWireSchema.safeParse(value);
  if (!parsed.success) {
    throw new Error('run detail contract validation failed');
  }

  return runDetailSnapshot(parsed.data);
}

export function runDetailSnapshot(detail: RunDetailWire): RunDetailSnapshot {
  return {
    run: runSnapshot(detail.run),
    events: detail.events.map(eventSnapshot),
    traces: detail.traces.map(traceSnapshot),
  };
}

function runSnapshot(run: RunDetailWire['run']): RunDetailSnapshot['run'] {
  return {
    id: run.id,
    shortId: shortRunId(run.id),
    agent: run.agent_id,
    kind: titleize(run.kind),
    status: titleize(run.status),
    externalId: run.external_id?.trim() || 'None',
    traces: run.trace_count,
    blocked: run.blocked_count,
    rewritten: run.rewritten_count,
    escalated: run.escalated_count,
    latency: run.p95_latency_ms === null ? 'No traces' : `${run.p95_latency_ms}ms`,
    started: relativeTime(new Date(run.started_at)),
    startedAt: formatDateTime(new Date(run.started_at)),
    endedAt: run.ended_at ? formatDateTime(new Date(run.ended_at)) : 'Still running',
    metadata: metadataEntries(run.metadata),
  };
}

function eventSnapshot(event: RunDetailWire['events'][number]): RunDetailSnapshot['events'][number] {
  return {
    id: event.id,
    sequence: event.sequence,
    kind: titleize(event.kind),
    label: event.label?.trim() || defaultEventLabel(event.kind, event.sequence),
    input: event.input_summary?.trim() || 'No input summary',
    output: event.output_summary?.trim() || 'No output summary',
    time: relativeTime(new Date(event.occurred_at)),
    metadata: metadataEntries(event.metadata),
  };
}

function traceSnapshot(trace: RunDetailWire['traces'][number]): RunDetailSnapshot['traces'][number] {
  return {
    id: trace.trace_id,
    runEventId: trace.run_event_id ?? null,
    phase: titleize(trace.domain),
    verdict: titleize(trace.decision),
    policy: readTracePolicy(trace.payload),
    reason: trace.payload.reason?.trim() || 'No reason recorded',
    safeOutput: trace.payload.safe_output?.trim() || null,
    latency: `${trace.elapsed_ms}ms`,
    time: relativeTime(new Date(trace.created_at)),
  };
}

function readTracePolicy(payload: RuntimeDecisionPayloadWire): string {
  const policy = payload.triggered_policies?.[0];
  return policy?.id?.trim() || 'baseline';
}

function metadataEntries(metadata: MetadataWire): Array<{ label: string; value: string }> {
  return Object.entries(metadata)
    .filter(([, value]) => value !== null && value !== '')
    .map(([label, value]) => ({ label, value: stringifyMetadataValue(value) }));
}

function stringifyMetadataValue(value: MetadataWire[keyof MetadataWire]): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

function titleize(value: string): string {
  return value
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function shortRunId(id: string): string {
  if (id.length <= 16) return id;
  return `${id.slice(0, 8)}...${id.slice(-4)}`;
}

function relativeTime(date: Date): string {
  const time = date.getTime();
  if (!Number.isFinite(time)) return 'Unknown';

  const seconds = Math.max(0, Math.round((Date.now() - time) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  return `${days}d ago`;
}

function formatDateTime(date: Date): string {
  const time = date.getTime();
  if (!Number.isFinite(time)) return 'Unknown';
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date);
}

function defaultEventLabel(kind: string, sequence: number): string {
  if (kind === 'user_turn' || kind === 'assistant_turn') return `Turn ${sequence}`;
  if (kind === 'workflow_step') return `Step ${sequence}`;
  return `Event ${sequence}`;
}
