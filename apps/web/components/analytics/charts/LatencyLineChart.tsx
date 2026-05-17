"use client";

import { CartesianGrid, Line, LineChart, XAxis, YAxis } from 'recharts';

import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';
import type { RunRow } from '@/lib/server/dashboard-data';

import { ChartCard } from '../ChartCard';
import { latencyConfig } from '../chartConfigs';
import { toLatencyPoints } from '../transforms';

export function LatencyLineChart({ runs }: { runs: RunRow[] }) {
  const data = toLatencyPoints(runs);

  return (
    <ChartCard title="Latency trend" description="p95 latency (ms) across recent runs">
      {data.length === 0 ? (
        <div className="border p-4 text-sm text-muted-foreground">No latency data recorded yet.</div>
      ) : (
        <ChartContainer config={latencyConfig} className="aspect-auto h-[260px] w-full">
          <LineChart data={data} margin={{ left: 0, right: 12 }}>
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
              tickFormatter={(v: number) => `${v}ms`}
              width={56}
            />
            <ChartTooltip
              content={
                <ChartTooltipContent
                  labelFormatter={(v) => `Run ${v}`}
                  formatter={(value) => [`${value}ms`, 'p95']}
                />
              }
            />
            <Line
              dataKey="p95Ms"
              stroke="var(--color-p95Ms)"
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4 }}
            />
          </LineChart>
        </ChartContainer>
      )}
    </ChartCard>
  );
}
