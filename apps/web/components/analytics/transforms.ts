// All client-side aggregation and data-shaping for the analytics chart suite.
// Every function is pure: RunRow[] in, typed chart data out.
// To add a new chart: add a transform here, a type in types.ts, a component in charts/.

import type { RunRow } from '@/lib/server/dashboard-data';
import type { HumanReviewAnalytics } from '@/lib/server/dashboard-data';
import type {
  AgentBreakdownRow,
  HumanReviewOutcomeRow,
  HumanReviewReasonRow,
  InterventionRatePoint,
  LatencyPoint,
  RunOutcomeRow,
  SummaryMetrics,
  VerdictTotal,
} from './types';

const CHART_WINDOW = 20; // max runs shown in per-run trend charts

export function toRunOutcomesRows(runs: RunRow[]): RunOutcomeRow[] {
  return runs
    .slice(0, CHART_WINDOW)
    .reverse()
    .map((run) => {
      const interventions = run.blocked + run.rewritten + run.escalated;
      return {
        run: run.shortId,
        allowed: Math.max(0, run.traces - interventions),
        blocked: run.blocked,
        rewritten: run.rewritten,
        escalated: run.escalated,
      };
    });
}

export function toLatencyPoints(runs: RunRow[]): LatencyPoint[] {
  return runs
    .slice(0, CHART_WINDOW)
    .reverse()
    .filter((run): run is RunRow & { p95LatencyMs: number } => run.p95LatencyMs !== null)
    .map((run) => ({ run: run.shortId, p95Ms: run.p95LatencyMs }));
}

export function toVerdictTotals(runs: RunRow[]): VerdictTotal[] {
  const totals = runs.reduce(
    (acc, run) => {
      const interventions = run.blocked + run.rewritten + run.escalated;
      return {
        allowed: acc.allowed + Math.max(0, run.traces - interventions),
        blocked: acc.blocked + run.blocked,
        rewritten: acc.rewritten + run.rewritten,
        escalated: acc.escalated + run.escalated,
      };
    },
    { allowed: 0, blocked: 0, rewritten: 0, escalated: 0 },
  );

  return (
    [
      { verdict: 'allowed', count: totals.allowed, fill: 'var(--color-allowed)' },
      { verdict: 'blocked', count: totals.blocked, fill: 'var(--color-blocked)' },
      { verdict: 'rewritten', count: totals.rewritten, fill: 'var(--color-rewritten)' },
      { verdict: 'escalated', count: totals.escalated, fill: 'var(--color-escalated)' },
    ] satisfies VerdictTotal[]
  ).filter((v) => v.count > 0);
}

export function toInterventionRatePoints(runs: RunRow[]): InterventionRatePoint[] {
  return runs
    .slice(0, CHART_WINDOW)
    .reverse()
    .map((run) => ({
      run: run.shortId,
      rate:
        run.traces === 0
          ? 0
          : Math.round(((run.blocked + run.rewritten + run.escalated) / run.traces) * 100),
    }));
}

export function toAgentBreakdown(runs: RunRow[]): AgentBreakdownRow[] {
  const byAgent = new Map<string, AgentBreakdownRow>();
  for (const run of runs) {
    const existing = byAgent.get(run.agent) ?? {
      agent: run.agent,
      traces: 0,
      blocked: 0,
      rewritten: 0,
      escalated: 0,
    };
    byAgent.set(run.agent, {
      ...existing,
      traces: existing.traces + run.traces,
      blocked: existing.blocked + run.blocked,
      rewritten: existing.rewritten + run.rewritten,
      escalated: existing.escalated + run.escalated,
    });
  }
  return Array.from(byAgent.values()).sort((a, b) => b.traces - a.traces);
}

export function toSummaryMetrics(
  runs: RunRow[],
  humanReviewAnalytics?: HumanReviewAnalytics,
): SummaryMetrics {
  const totals = runs.reduce(
    (acc, run) => ({
      traces: acc.traces + run.traces,
      interventions: acc.interventions + run.blocked + run.rewritten + run.escalated,
      p95: run.p95LatencyMs === null ? acc.p95 : Math.max(acc.p95, run.p95LatencyMs),
    }),
    { traces: 0, interventions: 0, p95: 0 },
  );

  return {
    runCount: runs.length,
    traceCount: totals.traces,
    guardrailInterventionCount: humanReviewAnalytics
      ? humanReviewAnalytics.summary.automatedInterventionCount
      : totals.interventions,
    humanInterventionCount: humanReviewAnalytics?.summary.humanInterventionCount ?? 0,
    humanInterventionRateLabel: humanReviewAnalytics
      ? `${humanReviewAnalytics.summary.humanInterventionRate}%`
      : '0%',
    p95LatencyLabel: totals.p95 === 0 ? 'No traces' : `${totals.p95}ms`,
  };
}

export function toHumanReviewOutcomeRows(
  analytics: HumanReviewAnalytics,
): HumanReviewOutcomeRow[] {
  return [
    { outcome: 'accepted', count: analytics.outcomes.acceptedCount },
    { outcome: 'corrected', count: analytics.outcomes.correctedCount },
    { outcome: 'rejected', count: analytics.outcomes.rejectedCount },
    { outcome: 'false positive', count: analytics.outcomes.falsePositiveCount },
    { outcome: 'missed issue', count: analytics.outcomes.missedIssueCount },
    { outcome: 'ignored', count: analytics.outcomes.ignoredCount },
  ].filter((row) => row.count > 0);
}

export function toHumanReviewReasonRows(
  analytics: HumanReviewAnalytics,
): HumanReviewReasonRow[] {
  return analytics.topReasons.slice(0, 8);
}
