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

const runtimeEventSchema = z
  .object({
    kind: z.string().optional(),
    action: z
      .object({
        operation: z.string().optional(),
        parameters: objectSchema.nullable().optional(),
        tool_identity: z
          .object({
            server_id: z.string().optional(),
            tool_name: z.string().optional(),
            schema_hash: z.string().optional(),
          })
          .passthrough()
          .optional(),
      })
      .passthrough()
      .optional(),
  })
  .passthrough();

const runtimeDecisionPayloadSchema = z
  .object({
    trace_id: z.string().optional(),
    effect: z.string().optional(),
    reason: z.string().optional(),
    triggered_policies: z.array(triggeredPolicySchema).optional(),
    safe_output: z.string().nullable().optional(),
    checked_input_excerpt: z.string().nullable().optional(),
    checked_output_excerpt: z.string().nullable().optional(),
    latency_ms: z.number().optional(),
    agent_id: z.string().optional(),
    event: runtimeEventSchema.optional(),
  })
  .passthrough();

const runSummarySchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  agent_id: z.string(),
  kind: z.string(),
  status: z.string(),
  evaluation_eligibility: z.enum(['eligible', 'legacy_incomplete']).default('eligible'),
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
  agent_id: z.string(),
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
  agent_id: z.string().nullable().optional(),
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

const runSpanSummarySchema = z.object({
  trace_id: z.string(),
  span_id: z.string(),
  parent_span_id: z.string().nullable(),
  agent_id: z.string(),
  run_event_id: z.string().nullable(),
  name: z.string(),
  span_kind: z.number(),
  operation_name: z.string().nullable(),
  conversation_id: z.string().nullable(),
  external_agent_id: z.string().nullable(),
  started_at: z.string(),
  ended_at: z.string(),
  status_code: z.number(),
  status_message: z.string().nullable(),
  resource: objectSchema,
  attributes: objectSchema,
  events: z.array(objectSchema),
  links: z.array(objectSchema),
  content_capture_status: z.string(),
  dropped_attribute_count: z.number(),
  late_evidence: z.boolean(),
  ingested_at: z.string(),
});

const providerUsageSchema = z.object({
  gateway_request_id: z.string(),
  route_id: z.string(),
  attempt: z.number().default(1),
  provider_connection_id: z.string().default(''),
  provider: z.string(),
  model: z.string(),
  provider_response_id: z.string().nullable(),
  status: z.string(),
  failure_code: z.string().nullable().default(null),
  prompt_tokens: z.number().nullable(),
  completion_tokens: z.number().nullable(),
  total_tokens: z.number().nullable(),
  latency_ms: z.number(),
  estimated_cost_usd_nanos: z.string().nullable(),
  input_rate_usd_per_million_nanos: z.string().nullable(),
  output_rate_usd_per_million_nanos: z.string().nullable(),
});

const coverageSchema = z.object({
  level: z.enum(['runtime_only', 'llm_boundary', 'llm_and_workflow', 'incomplete']),
  has_runtime_decisions: z.boolean(),
  has_llm_boundary: z.boolean(),
  has_workflow_evidence: z.boolean(),
  capture_complete: z.boolean(),
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

const runFinalizationSchema = z.object({
  finalized_at: z.string(),
  boundary_source: z.string(),
  boundary_confidence: z.string(),
  capture_status: z.string(),
  capture_deadline: z.string(),
  expected_flush_id: z.string().nullable().optional(),
});

const runParticipantSchema = z.object({
  agent_id: z.string(),
  role: z.string(),
  joined_at: z.string(),
});

const evaluationResultSchema = z.object({
  id: z.string(),
  run_id: z.string(),
  agent_id: z.string(),
  snapshot_hash: z.string(),
  manifest_hash: z.string(),
  evaluator_version: z.string(),
  verdict: z.string(),
  score_bps: z.number().nullable(),
  capture_status: z.string(),
  created_at: z.string(),
});

const evaluationJobSchema = z.object({
  id: z.string(),
  run_id: z.string(),
  agent_id: z.string(),
  status: z.string(),
  attempts: z.number(),
  error: z.string().nullable().optional(),
  updated_at: z.string(),
});

const runDetailWireSchema = z.object({
  run: runSummarySchema,
  events: z.array(runEventSummarySchema),
  traces: z.array(traceSummarySchema),
  spans: z.array(runSpanSummarySchema).default([]),
  provider_usage: providerUsageSchema.nullable().optional(),
  provider_attempts: z.array(providerUsageSchema).default([]),
  coverage: coverageSchema.default({
    level: 'runtime_only',
    has_runtime_decisions: false,
    has_llm_boundary: false,
    has_workflow_evidence: false,
    capture_complete: true,
  }),
  guardrail_usage: z.array(guardrailUsageSchema).default([]),
  budget_decision: budgetDecisionSchema.nullable().optional(),
  finalization: runFinalizationSchema.nullable().optional(),
  participants: z.array(runParticipantSchema).default([]),
  evaluation_jobs: z.array(evaluationJobSchema).default([]),
  evaluations: z.array(evaluationResultSchema).default([]),
});

type RuntimeDecisionPayloadWire = z.infer<typeof runtimeDecisionPayloadSchema>;
type RunDetailWire = z.infer<typeof runDetailWireSchema>;

export type RunAgentIdentity = {
  id: string;
  displayName: string | null;
  href: string | null;
};

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
    agentId: string | null;
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
    agentId: string | null;
    runEventId: string | null;
    side: TraceSide;
    phase: string;
    effect: string;
    outcome: string;
    triggered: boolean;
    severity: string | null;
    policy: string;
    reason: string;
    safeOutput: string | null;
    checkedInput: string | null;
    checkedOutput: string | null;
    operation: string | null;
    toolName: string | null;
    latency: string;
    time: string;
    clock: string;
    timestamp: number;
  }>;
  spans: Array<{
    key: string;
    traceId: string;
    spanId: string;
    parentSpanId: string | null;
    agentId: string;
    runEventId: string | null;
    name: string;
    kind: string;
    operation: string | null;
    conversationId: string | null;
    externalAgentId: string | null;
    service: string;
    startedAt: string;
    endedAt: string;
    startedMicros: number;
    endedMicros: number;
    durationMs: number;
    statusCode: number;
    status: string;
    statusMessage: string | null;
    resource: Array<{ label: string; value: string }>;
    attributes: Array<{ label: string; value: string }>;
    eventCount: number;
    linkCount: number;
    contentCaptureStatus: string;
    droppedAttributeCount: number;
    lateEvidence: boolean;
    ingestedAt: string;
  }>;
  providerUsage: z.infer<typeof providerUsageSchema> | null;
  providerAttempts: Array<z.infer<typeof providerUsageSchema>>;
  coverage: z.infer<typeof coverageSchema>;
  guardrailUsage: Array<z.infer<typeof guardrailUsageSchema>>;
  budgetDecision: z.infer<typeof budgetDecisionSchema> | null;
  assurance: {
    eligibility: 'eligible' | 'legacy_incomplete';
    finalization: z.infer<typeof runFinalizationSchema> | null;
    participants: Array<z.infer<typeof runParticipantSchema>>;
    jobs: Array<z.infer<typeof evaluationJobSchema>>;
    evaluations: Array<z.infer<typeof evaluationResultSchema>>;
  };
};

export type TraceSide = 'input' | 'output' | 'tool' | 'other';

export function currentAssuranceStatus(assurance: RunDetailSnapshot['assurance']): string {
  if (assurance.eligibility === 'legacy_incomplete') return assurance.eligibility;

  const agents = new Set([
    ...assurance.jobs.map((job) => job.agent_id),
    ...assurance.evaluations.map((evaluation) => evaluation.agent_id),
  ]);
  const current = [...agents].flatMap((agentId) => {
    const result = assurance.evaluations
      .filter((evaluation) => evaluation.agent_id === agentId)
      .sort((left, right) => Date.parse(right.created_at) - Date.parse(left.created_at))[0];
    const job = assurance.jobs
      .filter((candidate) => candidate.agent_id === agentId)
      .sort((left, right) => Date.parse(right.updated_at) - Date.parse(left.updated_at))[0];
    const jobIsNewer =
      job !== undefined &&
      (result === undefined || Date.parse(job.updated_at) > Date.parse(result.created_at));
    if (jobIsNewer && ['waiting_capture', 'queued', 'running', 'error'].includes(job.status)) {
      return [job.status];
    }
    if (result !== undefined) return [result.verdict];
    return job === undefined ? [] : [job.status];
  });
  const priority = [
    'failed',
    'error',
    'inconclusive',
    'not_configured',
    'running',
    'queued',
    'waiting_capture',
    'passed',
    'completed',
  ];
  return (
    priority.find((status) => current.includes(status)) ??
    assurance.finalization?.capture_status ??
    'not started'
  );
}

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
    spans: detail.spans.map(spanSnapshot),
    providerUsage: detail.provider_usage ?? null,
    providerAttempts: detail.provider_attempts,
    coverage: detail.coverage,
    guardrailUsage: detail.guardrail_usage,
    budgetDecision: detail.budget_decision ?? null,
    assurance: {
      eligibility: detail.run.evaluation_eligibility,
      finalization: detail.finalization ?? null,
      participants: detail.participants,
      jobs: detail.evaluation_jobs,
      evaluations: detail.evaluations,
    },
  };
}

function spanSnapshot(span: RunDetailWire['spans'][number]): RunDetailSnapshot['spans'][number] {
  const startedMicros = timestampMicros(span.started_at);
  const endedMicros = timestampMicros(span.ended_at);
  const resourceAttributes = span.resource['attributes'];
  const nestedServiceName =
    typeof resourceAttributes === 'object' &&
    resourceAttributes !== null &&
    !Array.isArray(resourceAttributes)
      ? Reflect.get(resourceAttributes, 'service.name')
      : null;
  const serviceName = span.resource['service.name'] ?? nestedServiceName;
  const service =
    (typeof serviceName === 'string' && serviceName.trim()) ||
    span.external_agent_id?.trim() ||
    span.agent_id;

  return {
    key: `${span.trace_id}:${span.span_id}`,
    traceId: span.trace_id,
    spanId: span.span_id,
    parentSpanId: span.parent_span_id,
    agentId: span.agent_id,
    runEventId: span.run_event_id,
    name: span.name,
    kind: spanKind(span.span_kind),
    operation: span.operation_name,
    conversationId: span.conversation_id,
    externalAgentId: span.external_agent_id,
    service,
    startedAt: span.started_at,
    endedAt: span.ended_at,
    startedMicros,
    endedMicros,
    durationMs: Math.max(0, endedMicros - startedMicros) / 1_000,
    statusCode: span.status_code,
    status: spanStatus(span.status_code),
    statusMessage: span.status_message,
    resource: metadataEntries(span.resource),
    attributes: metadataEntries(span.attributes),
    eventCount: span.events.length,
    linkCount: span.links.length,
    contentCaptureStatus: span.content_capture_status,
    droppedAttributeCount: span.dropped_attribute_count,
    lateEvidence: span.late_evidence,
    ingestedAt: span.ingested_at,
  };
}

function timestampMicros(value: string): number {
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) return parsed;
  const fraction = value.match(/\.(\d+)(?:Z|[+-]\d{2}:\d{2})$/)?.[1];
  if (!fraction) return parsed * 1_000;
  const microseconds = Number(`${fraction}000000`.slice(0, 6));
  return Math.floor(parsed / 1_000) * 1_000_000 + microseconds;
}

function spanKind(kind: number): string {
  return ['Unspecified', 'Internal', 'Server', 'Client', 'Producer', 'Consumer'][kind] ?? 'Unknown';
}

function spanStatus(status: number): string {
  return ['Unset', 'OK', 'Error'][status] ?? 'Unknown';
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
    agentId: event.agent_id,
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
  const eventKind = trace.payload.event?.kind;
  const action = trace.payload.event?.action;
  return {
    id: trace.trace_id,
    agentId: trace.agent_id ?? null,
    runEventId: trace.run_event_id ?? null,
    side: traceSide(trace.domain, eventKind),
    phase: titleize((eventKind ?? trace.domain).replaceAll('.', '_')),
    effect: titleize(trace.decision),
    outcome: trace.decision.toLowerCase(),
    triggered: (trace.payload.triggered_policies?.length ?? 0) > 0,
    severity: topPolicy?.severity?.trim() || null,
    policy: readTracePolicy(trace.payload),
    reason: trace.payload.reason?.trim() || 'No reason recorded',
    safeOutput: trace.payload.safe_output?.trim() || null,
    checkedInput:
      trace.payload.checked_input_excerpt?.trim() || formatActionParameters(trace.payload),
    checkedOutput: trace.payload.checked_output_excerpt?.trim() || null,
    operation: action?.operation?.trim() || null,
    toolName: action?.tool_identity?.tool_name?.trim() || null,
    latency: `${trace.elapsed_ms}ms`,
    time: relativeTime(createdAt),
    clock: formatClockTime(createdAt),
    timestamp: createdAt.getTime(),
  };
}

function traceSide(domain: string, eventKind?: string): TraceSide {
  const kind = eventKind?.toLowerCase();
  if (kind === 'tool.call.proposed' || kind === 'shell.action.proposed') return 'tool';
  if (kind === 'output.proposed') return 'output';
  const lower = domain.toLowerCase();
  if (lower.includes('input')) return 'input';
  if (lower.includes('output')) return 'output';
  return 'other';
}

function formatActionParameters(payload: RuntimeDecisionPayloadWire): string | null {
  const parameters = payload.event?.action?.parameters;
  if (parameters == null || Object.keys(parameters).length === 0) return null;
  return JSON.stringify(parameters, null, 2);
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
    agentId: inputTrace?.agent_id ?? outputTrace?.agent_id ?? null,
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
