"use client";

import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from 'recharts';

import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import type { RunRow } from '@/lib/server/dashboard-data';

import { ChartCard } from '../ChartCard';
import { verdictConfig } from '../chartConfigs';
import { toAgentBreakdown } from '../transforms';

export function AgentBreakdownBarChart({ runs }: { runs: RunRow[] }) {
  const data = toAgentBreakdown(runs);

  return (
    <ChartCard
      title="Agent breakdown"
      description="Traces and guardrail interventions grouped by agent across all loaded runs"
    >
      {data.length === 0 ? (
        <div className="border p-4 text-sm text-muted-foreground">No agent data yet.</div>
      ) : (
        <ChartContainer config={verdictConfig} className="aspect-auto h-[260px] w-full">
          <BarChart data={data} margin={{ left: 0, right: 12 }}>
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="agent"
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              minTickGap={8}
            />
            <YAxis tickLine={false} axisLine={false} tickMargin={8} width={40} />
            <ChartTooltip
              cursor={false}
              content={<ChartTooltipContent indicator="dot" />}
            />
            <ChartLegend content={<ChartLegendContent />} />
            <Bar dataKey="blocked" stackId="s" fill="var(--color-blocked)" radius={0} />
            <Bar dataKey="rewritten" stackId="s" fill="var(--color-rewritten)" radius={0} />
            <Bar
              dataKey="escalated"
              stackId="s"
              fill="var(--color-escalated)"
              radius={[4, 4, 0, 0]}
            />
          </BarChart>
        </ChartContainer>
      )}
    </ChartCard>
  );
}
