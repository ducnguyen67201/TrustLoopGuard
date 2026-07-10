import { z } from 'zod';

import {
  formatClockTime,
  formatDateTime,
  metadataEntries,
  relativeTime,
  shortRunId,
  titleize,
} from './run-format';

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
    checked_input_excerpt: z.string().nullable().optional(),
    checked_output_excerpt: z.string().nullable().optional(),
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

const providerUsageSchema = z.object({
  gateway_request_id: z.string(),
  route_id: z.string(),
  provider: z.string(),
  model: z.string(),
  provider_response_id: z.string().nullable(),
  status: z.string(),
  prompt_tokens: z.number().nullable(),
  completion_tokens: z.number().nullable(),
  total_tokens: z.number().nullable(),
  latency_ms: z.number(),
  estimated_cost_usd_nanos: z.string().nullable(),
  input_rate_usd_per_million_nanos: z.string().nullable(),
  output_rate_usd_per_million_nanos: z.string().nullable(),
});

const guardrailUsageSchema = z.object({
  phase: z.string(),
  judge: z.string(),
  provider: z.string().nullable(),
  model: z.string().nullable(),
  status: z.string(),
  prompt_tokens: z.number().nullable(),
  completion_tokens: z.number().nullable(),
  estimated_cost_usd_nanos: z.string().nullable(),
  fallback_used: z.boolean(),
  latency_ms: z.number(),
  error_code: z.string().nullable(),
});

const budgetWindowSchema = z.object({
  window: z.string(),
  cap_usd_nanos: z.string(),
  committed_before_usd_nanos: z.string(),
  reserved_before_usd_nanos: z.string(),
  requested_usd_nanos: z.string(),
  remaining_after_usd_nanos: z.string(),
});

const budgetDecisionSchema = z.object({
  principal_id: z.string(),
  status: z.string(),
  currency: z.string(),
  governing_window: z.string().nullable(),
  requested_usd_nanos: z.string().nullable(),
  actual_usd_nanos: z.string().nullable(),
  windows: z.array(budgetWindowSchema),
});

const runDetailWireSchema = z.object({
  run: runSummarySchema,
  events: z.array(runEventSummarySchema),
  traces: z.array(traceSummarySchema),
  provider_usage: providerUsageSchema.nullable().optional(),
  guardrail_usage: z.array(guardrailUsageSchema).default([]),
  budget_decision: budgetDecisionSchema.nullable().optional(),
});

type RuntimeDecisionPayloadWire = z.infer<typeof runtimeDecisionPayloadSchema>;
type RunDetailWire = z.infer<typeof runDetailWireSchema>;

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
    clock: string;
    timestamp: number;
    metadata: Array<{ label: string; value: string }>;
  }>;
  traces: Array<{
    id: string;
    runEventId: string | null;
    side: TraceSide;
    phase: string;
    verdict: string;
    outcome: string;
    triggered: boolean;
    severity: string | null;
    policy: string;
    reason: string;
    safeOutput: string | null;
    checkedInput: string | null;
    checkedOutput: string | null;
    latency: string;
    time: string;
    clock: string;
    timestamp: number;
  }>;
  providerUsage: z.infer<typeof providerUsageSchema> | null;
  guardrailUsage: Array<z.infer<typeof guardrailUsageSchema>>;
  budgetDecision: z.infer<typeof budgetDecisionSchema> | null;
};

export type TraceSide = 'input' | 'output' | 'other';

export function parseRunDetailSnapshot(value: Awaited<ReturnType<Response['json']>>) {
  const parsed = runDetailWireSchema.safeParse(value);
  if (!parsed.success) {
    throw new Error('run detail contract validation failed');
  }

  return runDetailSnapshot(parsed.data);
}

export function runDetailSnapshot(detail: RunDetailWire): RunDetailSnapshot {
  const traces = detail.traces.map(traceSnapshot);
  const events =
    detail.events.length > 0
      ? detail.events.map(eventSnapshot)
      : synthesizeGatewayTurnEvents(detail.traces, traces);

  return {
    run: runSnapshot(detail.run),
    events,
    traces,
    providerUsage: detail.provider_usage ?? null,
    guardrailUsage: detail.guardrail_usage,
    budgetDecision: detail.budget_decision ?? null,
  };
}

export function formatUsdNanos(value: string | null | undefined): string {
  if (value === null || value === undefined) return 'Unknown';
  try {
    const nanos = BigInt(value);
    const whole = nanos / 1_000_000_000n;
    const fractional = (nanos % 1_000_000_000n).toString().padStart(9, '0').replace(/0+$/, '');
    return `$${whole.toString()}${fractional ? `.${fractional}` : '.00'}`;
  } catch {
    return 'Unknown';
  }
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

function eventSnapshot(
  event: RunDetailWire['events'][number],
): RunDetailSnapshot['events'][number] {
  const occurredAt = new Date(event.occurred_at);
  return {
    id: event.id,
    sequence: event.sequence,
    kind: titleize(event.kind),
    label: event.label?.trim() || defaultEventLabel(event.kind, event.sequence),
    input: event.input_summary?.trim() || 'No input summary',
    output: event.output_summary?.trim() || 'No output summary',
    time: relativeTime(occurredAt),
    clock: formatClockTime(occurredAt),
    timestamp: occurredAt.getTime(),
    metadata: metadataEntries(event.metadata),
  };
}

function traceSnapshot(
  trace: RunDetailWire['traces'][number],
): RunDetailSnapshot['traces'][number] {
  const topPolicy = trace.payload.triggered_policies?.[0];
  const createdAt = new Date(trace.created_at);
  return {
    id: trace.trace_id,
    runEventId: trace.run_event_id ?? null,
    side: traceSide(trace.domain),
    phase: titleize(trace.domain),
    verdict: titleize(trace.decision),
    outcome: trace.decision.toLowerCase(),
    triggered: (trace.payload.triggered_policies?.length ?? 0) > 0,
    severity: topPolicy?.severity?.trim() || null,
    policy: readTracePolicy(trace.payload),
    reason: trace.payload.reason?.trim() || 'No reason recorded',
    safeOutput: trace.payload.safe_output?.trim() || null,
    checkedInput: trace.payload.checked_input_excerpt?.trim() || null,
    checkedOutput: trace.payload.checked_output_excerpt?.trim() || null,
    latency: `${trace.elapsed_ms}ms`,
    time: relativeTime(createdAt),
    clock: formatClockTime(createdAt),
    timestamp: createdAt.getTime(),
  };
}

function traceSide(domain: string): TraceSide {
  const lower = domain.toLowerCase();
  if (lower.includes('input')) return 'input';
  if (lower.includes('output')) return 'output';
  return 'other';
}

function synthesizeGatewayTurnEvents(
  wireTraces: RunDetailWire['traces'],
  traces: RunDetailSnapshot['traces'],
): RunDetailSnapshot['events'] {
  const byId = new Map(traces.map((trace) => [trace.id, trace]));
  const ordered = [...wireTraces].sort(
    (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
  );
  const events: RunDetailSnapshot['events'] = [];
  let pendingInput: RunDetailWire['traces'][number] | null = null;

  for (const trace of ordered) {
    if (trace.run_event_id) continue;

    if (trace.domain === 'gateway_input_check') {
      if (pendingInput) {
        events.push(syntheticGatewayEvent(events.length + 1, pendingInput, null, byId));
      }
      pendingInput = trace;
      continue;
    }

    if (trace.domain === 'gateway_output_check') {
      events.push(syntheticGatewayEvent(events.length + 1, pendingInput, trace, byId));
      pendingInput = null;
    }
  }

  if (pendingInput) {
    events.push(syntheticGatewayEvent(events.length + 1, pendingInput, null, byId));
  }

  return events;
}

function syntheticGatewayEvent(
  sequence: number,
  inputTrace: RunDetailWire['traces'][number] | null,
  outputTrace: RunDetailWire['traces'][number] | null,
  traceById: Map<string, RunDetailSnapshot['traces'][number]>,
): RunDetailSnapshot['events'][number] {
  const id = `gateway-turn-${inputTrace?.trace_id ?? 'none'}-${outputTrace?.trace_id ?? 'none'}`;
  const inputSnapshot = inputTrace ? traceById.get(inputTrace.trace_id) : null;
  const outputSnapshot = outputTrace ? traceById.get(outputTrace.trace_id) : null;
  if (inputSnapshot) inputSnapshot.runEventId = id;
  if (outputSnapshot) outputSnapshot.runEventId = id;

  const input =
    latestUserDisplayText(
      inputTrace?.payload.checked_input_excerpt ?? outputTrace?.payload.checked_input_excerpt,
    ) ?? 'No input summary';
  const output =
    outputTrace?.payload.checked_output_excerpt?.trim() ||
    inputTrace?.payload.checked_output_excerpt?.trim() ||
    'No output summary';
  const occurredAt = inputTrace?.created_at ?? outputTrace?.created_at ?? new Date().toISOString();
  const occurredDate = new Date(occurredAt);

  return {
    id,
    sequence,
    kind: 'User Turn',
    label: `Gateway turn ${sequence}`,
    input,
    output,
    time: relativeTime(occurredDate),
    clock: formatClockTime(occurredDate),
    timestamp: occurredDate.getTime(),
    metadata: [],
  };
}

function latestUserDisplayText(value: string | null | undefined): string | null {
  const text = value?.trim();
  if (!text) return null;

  const lines = text
    .split(/\n+/)
    .map((line) => line.trim())
    .filter(Boolean);
  const latestUserLine = [...lines]
    .reverse()
    .find((line) => line.toLowerCase().startsWith('user:'));
  const display = latestUserLine ? latestUserLine.replace(/^user:\s*/i, '') : text;
  return display.trim() || null;
}

function readTracePolicy(payload: RuntimeDecisionPayloadWire): string {
  const policy = payload.triggered_policies?.[0];
  return policy?.id?.trim() || 'baseline';
}

function defaultEventLabel(kind: string, sequence: number): string {
  if (kind === 'user_turn' || kind === 'assistant_turn') return `Turn ${sequence}`;
  if (kind === 'workflow_step') return `Step ${sequence}`;
  return `Event ${sequence}`;
}
