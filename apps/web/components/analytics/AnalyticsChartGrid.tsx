"use client";

import type { RunRow } from '@/lib/server/dashboard-data';

import { MetricCards } from './MetricCards';
import { toSummaryMetrics } from './transforms';
import { AgentBreakdownBarChart } from './charts/AgentBreakdownBarChart';
import { InterventionAreaChart } from './charts/InterventionAreaChart';
import { LatencyLineChart } from './charts/LatencyLineChart';
import { RunOutcomesBarChart } from './charts/RunOutcomesBarChart';
import { VerdictDonutChart } from './charts/VerdictDonutChart';

export function AnalyticsChartGrid({ runs }: { runs: RunRow[] }) {
  const metrics = toSummaryMetrics(runs);

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
    </div>
  );
}
