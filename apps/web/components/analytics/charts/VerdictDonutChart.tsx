"use client";

import { Cell, Label, Pie, PieChart } from 'recharts';

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
import { toVerdictTotals } from '../transforms';

export function VerdictDonutChart({ runs }: { runs: RunRow[] }) {
  const data = toVerdictTotals(runs);
  const total = data.reduce((sum, v) => sum + v.count, 0);

  return (
    <ChartCard title="Verdict distribution" description="Total trace outcomes across all loaded runs">
      {data.length === 0 ? (
        <div className="border p-4 text-sm text-muted-foreground">No verdict data yet.</div>
      ) : (
        <ChartContainer config={verdictConfig} className="aspect-auto h-[260px] w-full">
          <PieChart>
            <ChartTooltip
              content={
                <ChartTooltipContent
                  nameKey="verdict"
                  formatter={(value, name) => [value, verdictConfig[name as keyof typeof verdictConfig]?.label ?? name]}
                />
              }
            />
            <Pie
              data={data}
              dataKey="count"
              nameKey="verdict"
              innerRadius={60}
              outerRadius={90}
              strokeWidth={2}
            >
              {data.map((entry) => (
                <Cell key={entry.verdict} fill={entry.fill} />
              ))}
              <Label
                content={({ viewBox }) => {
                  if (!viewBox || !('cx' in viewBox) || !('cy' in viewBox)) return null;
                  return (
                    <text
                      x={viewBox.cx}
                      y={viewBox.cy}
                      textAnchor="middle"
                      dominantBaseline="middle"
                    >
                      <tspan
                        x={viewBox.cx}
                        y={viewBox.cy}
                        className="fill-foreground text-2xl font-bold"
                      >
                        {total.toLocaleString()}
                      </tspan>
                      <tspan
                        x={viewBox.cx}
                        y={(viewBox.cy ?? 0) + 20}
                        className="fill-muted-foreground text-xs"
                      >
                        traces
                      </tspan>
                    </text>
                  );
                }}
              />
            </Pie>
            <ChartLegend content={<ChartLegendContent nameKey="verdict" />} />
          </PieChart>
        </ChartContainer>
      )}
    </ChartCard>
  );
}
