'use client';

import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from 'recharts';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ChartContainer, ChartTooltip, ChartTooltipContent } from '@/components/ui/chart';
import { EmptyState } from '@/components/ui/empty-state';

import { formatMinorUnits } from '../financial-utils';
import { periodLabel, USAGE_CURRENCY, type UsagePeriod } from './usage-utils';

const chartConfig = {
  cost: { label: 'Spend', color: 'var(--chart-1)' },
};

type SpendOverTimeCardProps = {
  dayBuckets: LlmUsageBucket[];
  period: UsagePeriod;
  className?: string;
};

/** Daily spend bar chart for the active period. Fixed-height so nothing shifts. */
export function SpendOverTimeCard({ dayBuckets, period, className }: SpendOverTimeCardProps) {
  const chartData = dayBuckets.map((bucket) => ({
    day: bucket.key,
    cost: Number(bucket.cost_minor) / 100,
  }));

  return (
    <Card className={className}>
      <CardHeader>
        <CardDescription>{periodLabel(period)}, per day</CardDescription>
        <CardTitle>Spend over time</CardTitle>
      </CardHeader>
      <CardContent>
        {chartData.length === 0 ? (
          <EmptyState
            className="h-64"
            title="No spend in this period"
            description="Metered gateway calls plot here by day."
          />
        ) : (
          <ChartContainer config={chartConfig} className="aspect-auto h-64 w-full">
            <BarChart data={chartData} margin={{ left: 0, right: 12 }}>
              <CartesianGrid vertical={false} />
              <XAxis
                dataKey="day"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                minTickGap={18}
              />
              <YAxis
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                width={64}
                tickFormatter={(value: number) =>
                  formatMinorUnits(Math.round(value * 100), USAGE_CURRENCY)
                }
              />
              <ChartTooltip content={<ChartTooltipContent />} />
              <Bar dataKey="cost" fill="var(--color-cost)" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ChartContainer>
        )}
      </CardContent>
    </Card>
  );
}
