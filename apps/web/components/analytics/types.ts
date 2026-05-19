// Chart-layer data shapes. These are the output types of transforms.ts — decoupled from the
// domain RunRow type so chart components never import from the server data layer directly.

export type RunOutcomeRow = {
  run: string;
  allowed: number;
  blocked: number;
  rewritten: number;
  escalated: number;
};

export type LatencyPoint = {
  run: string;
  p95Ms: number;
};

export type VerdictTotal = {
  verdict: string;
  count: number;
  fill: string; // CSS var, e.g. "var(--color-allowed)"
};

export type InterventionRatePoint = {
  run: string;
  rate: number; // 0–100 (percentage)
};

export type AgentBreakdownRow = {
  agent: string;
  traces: number;
  blocked: number;
  rewritten: number;
  escalated: number;
};

export type SummaryMetrics = {
  runCount: number;
  traceCount: number;
  guardrailInterventionCount: number;
  humanInterventionCount: number;
  humanInterventionRateLabel: string;
  p95LatencyLabel: string;
};

export type HumanReviewOutcomeRow = {
  outcome: string;
  count: number;
};

export type HumanReviewReasonRow = {
  reasonCode: string;
  count: number;
};
