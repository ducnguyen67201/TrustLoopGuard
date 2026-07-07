'use client';

import Link from 'next/link';
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from 'recharts';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ChartContainer, ChartTooltip, ChartTooltipContent } from '@/components/ui/chart';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { PageHeader } from '@/components/ui/page-header';
import { currentContextQuery, formatMinorUnits, titleLabel } from './financial-utils';
import { formatTokens, USAGE_PERIODS, type UsagePeriod } from './usage-utils';

// Gateway pricing is USD minor units (crates/tl-server/src/llm_pricing.rs);
// usage buckets carry no currency field.
const USAGE_CURRENCY = 'USD';

const chartConfig = {
  cost: { label: 'Spend', color: 'var(--chart-1)' },
};

type UsageContentProps = {
  workspaceSlug: string;
  environmentId: string;
  period: UsagePeriod;
  dayBuckets: LlmUsageBucket[];
  principalBuckets: LlmUsageBucket[];
  modelBuckets: LlmUsageBucket[];
};

export function UsageContent({
  workspaceSlug,
  environmentId,
  period,
  dayBuckets,
  principalBuckets,
  modelBuckets,
}: UsageContentProps) {
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);
  const totalSpendMinor = sum(principalBuckets, (bucket) => bucket.cost_minor);
  const totalTokens =
    sum(principalBuckets, (bucket) => bucket.prompt_tokens) +
    sum(principalBuckets, (bucket) => bucket.completion_tokens);
  const hasUsage =
    dayBuckets.length > 0 || principalBuckets.length > 0 || modelBuckets.length > 0;

  const chartData = dayBuckets.map((bucket) => ({
    day: bucket.key,
    cost: Number(bucket.cost_minor) / 100,
  }));

  const principalColumns: DataTableColumn<LlmUsageBucket>[] = [
    {
      id: 'principal',
      header: 'Principal',
      cell: (row) => <span className="truncate font-mono text-xs">{row.key}</span>,
    },
    {
      id: 'prompt-tokens',
      header: 'Prompt tokens',
      align: 'right',
      cell: (row) => (
        <span className="font-mono text-sm tabular-nums">{formatTokens(row.prompt_tokens)}</span>
      ),
    },
    {
      id: 'completion-tokens',
      header: 'Completion tokens',
      align: 'right',
      cell: (row) => (
        <span className="font-mono text-sm tabular-nums">
          {formatTokens(row.completion_tokens)}
        </span>
      ),
    },
    {
      id: 'calls',
      header: 'Calls',
      align: 'right',
      cell: (row) => (
        <span className="font-mono text-sm tabular-nums">{Number(row.calls).toLocaleString('en-US')}</span>
      ),
    },
    {
      id: 'cost',
      header: 'Spend',
      align: 'right',
      cell: (row) => (
        <span className="font-mono text-sm tabular-nums">
          {formatMinorUnits(row.cost_minor, USAGE_CURRENCY)}
        </span>
      ),
    },
  ];

  const modelColumns: DataTableColumn<LlmUsageBucket>[] = [
    {
      id: 'model',
      header: 'Model',
      cell: (row) => <span className="truncate font-mono text-xs">{row.key}</span>,
    },
    {
      id: 'calls',
      header: 'Calls',
      align: 'right',
      cell: (row) => (
        <span className="font-mono text-sm tabular-nums">{Number(row.calls).toLocaleString('en-US')}</span>
      ),
    },
    {
      id: 'tokens',
      header: 'Tokens',
      align: 'right',
      cell: (row) => (
        <span className="font-mono text-sm tabular-nums">
          {formatTokens(Number(row.prompt_tokens) + Number(row.completion_tokens))}
        </span>
      ),
    },
    {
      id: 'cost',
      header: 'Spend',
      align: 'right',
      cell: (row) => (
        <span className="font-mono text-sm tabular-nums">
          {formatMinorUnits(row.cost_minor, USAGE_CURRENCY)}
        </span>
      ),
    },
  ];

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Monitor"
        title="Usage"
        description="Token consumption and spend per principal and model, metered at the LLM gateway."
        actions={<PeriodSelector period={period} contextQuery={contextQuery} />}
      />
      <div className="grid gap-3 md:grid-cols-3">
        <SummaryTile label="Total spend" value={formatMinorUnits(totalSpendMinor, USAGE_CURRENCY)} />
        <SummaryTile label="Total tokens" value={formatTokens(totalTokens)} />
        <SummaryTile label="Active principals" value={principalBuckets.length.toLocaleString('en-US')} />
      </div>
      {!hasUsage ? (
        <EmptyState
          title="No usage yet"
          description="Point an agent at the gateway — metered LLM calls will show up here."
        />
      ) : (
        <>
          <Card>
            <CardHeader>
              <CardTitle>Spend over time</CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="aspect-auto h-64 w-full">
                <BarChart data={chartData} margin={{ left: 0, right: 12 }}>
                  <CartesianGrid vertical={false} />
                  <XAxis dataKey="day" tickLine={false} axisLine={false} tickMargin={8} minTickGap={18} />
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
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>By principal</CardTitle>
            </CardHeader>
            <CardContent>
              <DataTable
                columns={principalColumns}
                rows={principalBuckets}
                getRowKey={(row) => row.key}
                empty={<EmptyState title="No principal usage in this period" />}
                caption="LLM usage by principal"
              />
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>By model</CardTitle>
            </CardHeader>
            <CardContent>
              <DataTable
                columns={modelColumns}
                rows={modelBuckets}
                getRowKey={(row) => row.key}
                empty={<EmptyState title="No model usage in this period" />}
                caption="LLM usage by model"
              />
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}

function PeriodSelector({ period, contextQuery }: { period: UsagePeriod; contextQuery: string }) {
  return (
    <div
      className="flex items-center gap-1 rounded-lg border bg-card p-1"
      role="group"
      aria-label="Usage period"
    >
      {USAGE_PERIODS.map((option) => (
        <Button key={option} asChild size="sm" variant={option === period ? 'secondary' : 'ghost'}>
          <Link
            href={`/usage${contextQuery}&period=${option}`}
            aria-current={option === period ? 'page' : undefined}
          >
            {titleLabel(option)}
          </Link>
        </Button>
      ))}
    </div>
  );
}

function SummaryTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border bg-card px-4 py-3">
      <p className="text-xs uppercase text-muted-foreground">{label}</p>
      <p className="mt-1 font-mono text-2xl font-semibold tabular-nums text-foreground">{value}</p>
    </div>
  );
}

function sum(buckets: LlmUsageBucket[], pick: (bucket: LlmUsageBucket) => number | bigint): number {
  return buckets.reduce((total, bucket) => total + Number(pick(bucket)), 0);
}
