"use client";

import { Bar, BarChart, CartesianGrid, XAxis } from 'recharts';

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
import { toRunOutcomesRows } from '../transforms';

export function RunOutcomesBarChart({ runs }: { runs: RunRow[] }) {
  const data = toRunOutcomesRows(runs);

  return (
    <ChartCard title="Run outcomes" description="Guardrail verdict breakdown per recent run">
      {data.length === 0 ? (
        <div className="border p-4 text-sm text-muted-foreground">No runs recorded yet.</div>
      ) : (
        <ChartContainer config={verdictConfig} className="aspect-auto h-[320px] w-full">
          <BarChart data={data} margin={{ left: 0, right: 12 }}>
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
                <ChartTooltipContent indicator="dot" labelFormatter={(v) => `Run ${v}`} />
              }
            />
            <ChartLegend content={<ChartLegendContent />} />
            <Bar dataKey="allowed" stackId="s" fill="var(--color-allowed)" radius={0} />
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
