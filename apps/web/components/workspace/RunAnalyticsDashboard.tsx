"use client";

import { Bar, BarChart, CartesianGrid, XAxis } from 'recharts';

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from '@/components/ui/chart';

export type RunAnalyticsRun = {
  id: string;
  shortId: string;
  agent: string;
  traces: number;
  blocked: number;
  rewritten: number;
  escalated: number;
  p95LatencyMs: number | null;
};

type RunAnalyticsDashboardProps = {
  runs: RunAnalyticsRun[];
};

type ChartRow = {
  run: string;
  allowed: number;
  blocked: number;
  rewritten: number;
  escalated: number;
};

const chartConfig = {
  allowed: {
    label: 'Allowed',
    color: 'var(--chart-2)',
  },
  blocked: {
    label: 'Blocked',
    color: 'var(--chart-5)',
  },
  rewritten: {
    label: 'Rewritten',
    color: 'var(--chart-4)',
  },
  escalated: {
    label: 'Escalated',
    color: 'var(--chart-1)',
  },
} satisfies ChartConfig;

export function RunAnalyticsDashboard({ runs }: RunAnalyticsDashboardProps) {
  const chartRows = runs.slice(0, 12).reverse().map(chartRow);
  const totals = aggregateRuns(runs);

  return (
    <div className="grid gap-4">
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <MetricCard label="Runs" value={String(runs.length)} detail="Recent customer sessions" />
        <MetricCard
          label="Traces"
          value={String(totals.traces)}
          detail="Guardrail checks in sample"
        />
        <MetricCard
          label="Interventions"
          value={String(totals.interventions)}
          detail="Blocked, rewritten, or escalated"
        />
        <MetricCard label="p95 latency" value={totals.p95Latency} detail="Highest recent run p95" />
      </div>
      <Card>
        <CardHeader>
          <CardDescription>Recent customer sessions</CardDescription>
          <CardTitle>Run outcomes</CardTitle>
        </CardHeader>
        <CardContent>
          {chartRows.length === 0 ? (
            <div className="border p-4 text-sm text-muted-foreground">
              No runs recorded in this workspace yet.
            </div>
          ) : (
            <ChartContainer config={chartConfig} className="aspect-auto h-[320px] w-full">
              <BarChart data={chartRows} margin={{ left: 0, right: 12 }}>
                <CartesianGrid vertical={false} />
                <XAxis
                  dataKey="run"
                  tickLine={false}
                  axisLine={false}
                  tickMargin={8}
                  minTickGap={20}
                />
                <ChartTooltip
                  cursor={false}
                  content={
                    <ChartTooltipContent
                      indicator="dot"
                      labelFormatter={(value) => `Run ${value}`}
                    />
                  }
                />
                <Bar dataKey="allowed" stackId="outcomes" fill="var(--color-allowed)" radius={0} />
                <Bar dataKey="blocked" stackId="outcomes" fill="var(--color-blocked)" radius={0} />
                <Bar dataKey="rewritten" stackId="outcomes" fill="var(--color-rewritten)" radius={0} />
                <Bar
                  dataKey="escalated"
                  stackId="outcomes"
                  fill="var(--color-escalated)"
                  radius={[4, 4, 0, 0]}
                />
              </BarChart>
            </ChartContainer>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function MetricCard({
  label,
  value,
  detail,
}: {
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <Card>
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle className="text-2xl">{value}</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function chartRow(run: RunAnalyticsRun): ChartRow {
  const interventionCount = run.blocked + run.rewritten + run.escalated;
  return {
    run: run.shortId,
    allowed: Math.max(0, run.traces - interventionCount),
    blocked: run.blocked,
    rewritten: run.rewritten,
    escalated: run.escalated,
  };
}

function aggregateRuns(runs: RunAnalyticsRun[]) {
  const totals = runs.reduce(
    (acc, run) => ({
      traces: acc.traces + run.traces,
      interventions: acc.interventions + run.blocked + run.rewritten + run.escalated,
      p95LatencyMs:
        run.p95LatencyMs === null
          ? acc.p95LatencyMs
          : Math.max(acc.p95LatencyMs, run.p95LatencyMs),
    }),
    { traces: 0, interventions: 0, p95LatencyMs: 0 },
  );

  return {
    ...totals,
    p95Latency: totals.p95LatencyMs === 0 ? 'No traces' : `${totals.p95LatencyMs}ms`,
  };
}
