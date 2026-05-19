"use client";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';

import type { SummaryMetrics } from './types';

export function MetricCards({ metrics }: { metrics: SummaryMetrics }) {
  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <MetricCard label="Runs" value={String(metrics.runCount)} detail="Recent sessions loaded" />
      <MetricCard label="Traces" value={String(metrics.traceCount)} detail="Guardrail checks in sample" />
      <MetricCard
        label="Guardrail interventions"
        value={String(metrics.guardrailInterventionCount)}
        detail="Blocked, rewritten, or escalated"
      />
      <MetricCard
        label="Human intervention rate"
        value={metrics.humanInterventionRateLabel}
        detail={`${metrics.humanInterventionCount} corrected, rejected, or missed issue outcomes`}
      />
    </div>
  );
}

function MetricCard({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <Card>
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle className="text-2xl tabular-nums">{value}</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}
