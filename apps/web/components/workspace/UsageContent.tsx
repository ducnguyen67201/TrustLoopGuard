'use client';

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useState } from 'react';
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from 'recharts';
import { toast } from 'sonner';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ChartContainer, ChartTooltip, ChartTooltipContent } from '@/components/ui/chart';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { currentContextQuery, formatMinorUnits, safeError, titleLabel } from './financial-utils';
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
  const [priceModel, setPriceModel] = useState<string | null>(null);
  const unpricedBuckets = modelBuckets.filter((bucket) => bucket.unpriced);
  const unpricedCalls = sum(unpricedBuckets, (bucket) => bucket.calls);
  const totalSpendMinor = sum(principalBuckets, (bucket) => bucket.cost_minor);
  const totalTokens =
    sum(principalBuckets, (bucket) => bucket.prompt_tokens) +
    sum(principalBuckets, (bucket) => bucket.completion_tokens);
  const hasUsage = dayBuckets.length > 0 || principalBuckets.length > 0 || modelBuckets.length > 0;

  const chartData = dayBuckets.map((bucket) => ({
    day: bucket.key,
    cost: Number(bucket.cost_minor) / 100,
  }));

  const principalColumns: DataTableColumn<LlmUsageBucket>[] = [
    {
      id: 'principal',
      header: 'Caller',
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
        <span className="font-mono text-sm tabular-nums">
          {Number(row.calls).toLocaleString('en-US')}
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

  const modelColumns: DataTableColumn<LlmUsageBucket>[] = [
    {
      id: 'model',
      header: 'Model',
      cell: (row) => (
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate font-mono text-xs">{row.key}</span>
          {row.unpriced ? (
            <>
              <Badge variant="outline" className="border-destructive/50 text-destructive">
                No price
              </Badge>
              <Button size="sm" variant="ghost" onClick={() => setPriceModel(row.key)}>
                Set price
              </Button>
            </>
          ) : null}
        </div>
      ),
    },
    {
      id: 'calls',
      header: 'Calls',
      align: 'right',
      cell: (row) => (
        <span className="font-mono text-sm tabular-nums">
          {Number(row.calls).toLocaleString('en-US')}
        </span>
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
    <section aria-labelledby="usage-overview-title" className="grid gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div className="grid gap-1">
          <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
            LLM gateway usage
          </p>
          <h2 id="usage-overview-title" className="text-2xl font-semibold tracking-tight">
            Usage
          </h2>
          <p className="max-w-3xl text-sm text-muted-foreground">
            Token consumption and spend by caller and model, metered at the LLM gateway.
          </p>
        </div>
        <PeriodSelector period={period} contextQuery={contextQuery} />
      </div>
      <div className="grid gap-3 md:grid-cols-3">
        <SummaryTile
          label="Total spend"
          value={formatMinorUnits(totalSpendMinor, USAGE_CURRENCY)}
        />
        <SummaryTile label="Total tokens" value={formatTokens(totalTokens)} />
        <SummaryTile
          label="Active callers"
          value={principalBuckets.length.toLocaleString('en-US')}
        />
      </div>
      {!hasUsage ? (
        <EmptyState
          title="No usage yet"
          description="Point an agent at the gateway — metered LLM calls will show up here."
        />
      ) : (
        <>
          {unpricedBuckets.length > 0 ? (
            <div
              role="status"
              className="rounded-lg border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm"
            >
              <span className="font-medium text-destructive">
                {formatTokens(unpricedCalls)} call{unpricedCalls === 1 ? '' : 's'} across{' '}
                {unpricedBuckets.length} model{unpricedBuckets.length === 1 ? '' : 's'} have no
                price set — spend is undercounted.
              </span>{' '}
              <span className="text-muted-foreground">
                Set a price on the flagged models below to meter them.
              </span>
            </div>
          ) : null}
          <Card>
            <CardHeader>
              <CardTitle>Spend over time</CardTitle>
            </CardHeader>
            <CardContent>
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
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>By caller</CardTitle>
            </CardHeader>
            <CardContent>
              <DataTable
                columns={principalColumns}
                rows={principalBuckets}
                getRowKey={(row) => row.key}
                empty={<EmptyState title="No caller usage in this period" />}
                caption="LLM usage by caller"
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
      <SetPriceDialog
        model={priceModel}
        contextQuery={contextQuery}
        onOpenChange={(open) => {
          if (!open) setPriceModel(null);
        }}
      />
    </section>
  );
}

function SetPriceDialog({
  model,
  contextQuery,
  onOpenChange,
}: {
  model: string | null;
  contextQuery: string;
  onOpenChange: (open: boolean) => void;
}) {
  const router = useRouter();
  const [saving, setSaving] = useState(false);
  const [inputPrice, setInputPrice] = useState('');
  const [outputPrice, setOutputPrice] = useState('');

  async function savePrice() {
    if (!model) return;
    let payload: { input_per_million_minor: number; output_per_million_minor: number };
    try {
      payload = {
        input_per_million_minor: dollarsToMinor(inputPrice),
        output_per_million_minor: dollarsToMinor(outputPrice),
      };
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Price is invalid');
      return;
    }
    setSaving(true);
    try {
      const response = await fetch(`/api/llm-pricing/${encodeURIComponent(model)}${contextQuery}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const text = await response.text();
      if (!response.ok) {
        throw new Error(safeError(text) ?? 'Unable to set model price');
      }
      toast.success(`Price set for ${model}`);
      onOpenChange(false);
      setInputPrice('');
      setOutputPrice('');
      router.refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Unable to set model price');
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={model !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Set model price</DialogTitle>
          <DialogDescription>
            USD per 1M tokens for <span className="font-mono">{model}</span>. New calls are metered
            at this workspace price immediately.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3 md:grid-cols-2">
          <div className="grid gap-1.5">
            <Label htmlFor="set-price-input">Input $ per 1M tokens</Label>
            <Input
              id="set-price-input"
              inputMode="decimal"
              value={inputPrice}
              onChange={(event) => setInputPrice(event.target.value)}
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="set-price-output">Output $ per 1M tokens</Label>
            <Input
              id="set-price-output"
              inputMode="decimal"
              value={outputPrice}
              onChange={(event) => setOutputPrice(event.target.value)}
            />
          </div>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" disabled={saving} onClick={savePrice}>
            {saving ? 'Saving...' : 'Set price'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function dollarsToMinor(value: string): number {
  const trimmed = value.trim();
  if (!/^\d+(\.\d{1,2})?$/.test(trimmed)) {
    throw new Error('Prices must be non-negative dollars with up to two decimals');
  }
  const [dollars, cents = ''] = trimmed.split('.');
  return Number(dollars) * 100 + Number(cents.padEnd(2, '0'));
}

function PeriodSelector({
  period,
  contextQuery,
}: {
  period: UsagePeriod;
  contextQuery: string;
}) {
  return (
    <div
      className="flex items-center gap-1 rounded-lg border bg-card p-1"
      role="group"
      aria-label="Usage period"
    >
      {USAGE_PERIODS.map((option) => (
        <Button key={option} asChild size="sm" variant={option === period ? 'secondary' : 'ghost'}>
          <Link
            href={`/${contextQuery}&period=${option}#usage-overview-title`}
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
