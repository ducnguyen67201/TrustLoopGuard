"use client";

import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts';

import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import type { RunRow } from '@/lib/server/dashboard-data';

import { ChartCard } from '../ChartCard';
import { interventionRateConfig } from '../chartConfigs';
import { toInterventionRatePoints } from '../transforms';

export function InterventionAreaChart({ runs }: { runs: RunRow[] }) {
  const data = toInterventionRatePoints(runs);

  return (
    <ChartCard
      title="Intervention rate"
      description="Percentage of traces resulting in block, rewrite, or escalation per run"
    >
      {data.length === 0 ? (
        <div className="border p-4 text-sm text-muted-foreground">No runs recorded yet.</div>
      ) : (
        <ChartContainer config={interventionRateConfig} className="aspect-auto h-[260px] w-full">
          <AreaChart data={data} margin={{ left: 0, right: 12 }}>
            <defs>
              <linearGradient id="fillRate" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="var(--color-rate)" stopOpacity={0.4} />
                <stop offset="95%" stopColor="var(--color-rate)" stopOpacity={0.05} />
              </linearGradient>
            </defs>
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="run"
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              minTickGap={20}
            />
            <YAxis
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              domain={[0, 100]}
              tickFormatter={(v: number) => `${v}%`}
              width={44}
            />
            <ChartTooltip
              content={
                <ChartTooltipContent
                  labelFormatter={(v) => `Run ${v}`}
                  formatter={(value) => [`${value}%`, 'Intervention rate']}
                />
              }
            />
            <Area
              dataKey="rate"
              stroke="var(--color-rate)"
              strokeWidth={2}
              fill="url(#fillRate)"
              dot={false}
              activeDot={{ r: 4 }}
            />
          </AreaChart>
        </ChartContainer>
      )}
    </ChartCard>
  );
}
