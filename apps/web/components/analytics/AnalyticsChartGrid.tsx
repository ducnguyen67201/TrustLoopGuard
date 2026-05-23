"use client";

import type { RunRow } from '@/lib/server/dashboard-data';
import type { HumanReviewAnalytics } from '@/lib/server/dashboard-data';

import { MetricCards } from './MetricCards';
import {
  toHumanReviewOutcomeRows,
  toHumanReviewReasonRows,
  toSummaryMetrics,
} from './transforms';
import { AgentBreakdownBarChart } from './charts/AgentBreakdownBarChart';
import { InterventionAreaChart } from './charts/InterventionAreaChart';
import { LatencyLineChart } from './charts/LatencyLineChart';
import { RunOutcomesBarChart } from './charts/RunOutcomesBarChart';
import { VerdictDonutChart } from './charts/VerdictDonutChart';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function AnalyticsChartGrid({
  runs,
  humanReviewAnalytics,
}: {
  runs: RunRow[];
  humanReviewAnalytics: HumanReviewAnalytics;
}) {
  const metrics = toSummaryMetrics(runs, humanReviewAnalytics);
  const outcomeRows = toHumanReviewOutcomeRows(humanReviewAnalytics);
  const reasonRows = toHumanReviewReasonRows(humanReviewAnalytics);

  return (
    <div className="grid gap-4">
      <MetricCards metrics={metrics} />

      <RunOutcomesBarChart runs={runs} />

      <div className="grid gap-4 md:grid-cols-2">
        <LatencyLineChart runs={runs} />
        <VerdictDonutChart runs={runs} />
      </div>

      <InterventionAreaChart runs={runs} />

      <AgentBreakdownBarChart runs={runs} />

      <div className="grid gap-4 md:grid-cols-2">
        <AnalyticsTable
          title="Human review outcomes"
          description="Latest recorded outcome per reviewed trace"
          rows={outcomeRows.map((row) => ({ label: row.outcome, value: row.count }))}
          empty="No review outcomes recorded."
        />
        <AnalyticsTable
          title="Top review reasons"
          description="Reason codes from latest recorded outcomes"
          rows={reasonRows.map((row) => ({ label: row.reasonCode, value: row.count }))}
          empty="No review reason codes recorded."
        />
      </div>
    </div>
  );
}

function AnalyticsTable({
  title,
  description,
  rows,
  empty,
}: {
  title: string;
  description: string;
  rows: Array<{ label: string; value: number }>;
  empty: string;
}) {
  return (
    <Card>
      <CardHeader>
        <CardDescription>{description}</CardDescription>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>
        {rows.length === 0 ? (
          <div className="border p-4 text-sm text-muted-foreground">{empty}</div>
        ) : (
          <div className="grid gap-2">
            {rows.map((row) => (
              <div key={row.label} className="flex items-center justify-between gap-4 text-sm">
                <span className="text-muted-foreground">{row.label}</span>
                <span className="font-medium tabular-nums">{row.value}</span>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
