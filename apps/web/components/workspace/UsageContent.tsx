'use client';

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useState } from 'react';
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from 'recharts';
import { toast } from 'sonner';
import type { LlmUsageBucket } from '@featherlane-ai/sdk';
import { z } from 'zod';

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
import { formatUsdNanos } from '@/lib/run-detail-live';

// Gateway pricing is USD minor units (crates/tl-server/src/llm_pricing.rs);
// usage buckets carry no currency field.
const USAGE_CURRENCY = 'USD';

const chartConfig = {
  cost: { label: 'Spend', color: 'var(--chart-1)' },
};

const priceRateSchema = z
  .string()
  .trim()
  .regex(/^\d+(\.\d{1,9})?$/, 'Prices must be non-negative dollars with up to nine decimals');

type UsageContentProps = {
  workspaceSlug: string;
  environmentId: string;
  period: UsagePeriod;
  dayBuckets: LlmUsageBucket[];
  principalBuckets: LlmUsageBucket[];
  modelBuckets: LlmUsageBucket[];
  guardrailBuckets?: LlmUsageBucket[];
};

export function UsageContent({
  workspaceSlug,
  environmentId,
  period,
  dayBuckets,
  principalBuckets,
  modelBuckets,
  guardrailBuckets = [],
}: UsageContentProps) {
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);
  const [priceModel, setPriceModel] = useState<string | null>(null);
  const unpricedBuckets = modelBuckets.filter((bucket) => bucket.unpriced);
  const unpricedCalls = sum(unpricedBuckets, (bucket) => bucket.calls);
  const totalSpendNanos = sumNanos(principalBuckets);
  const totalSpend =
    unpricedBuckets.length === 0
      ? formatUsdNanos(totalSpendNanos.toString())
      : totalSpendNanos === 0n
        ? 'Unknown'
        : `${formatUsdNanos(totalSpendNanos.toString())} + unknown`;
  const guardrailCostNanos = guardrailBuckets.reduce(
    (total, bucket) => total + BigInt(bucket.cost_usd_nanos),
    0n,
  );
  const hasUnpricedGuardrailUsage = guardrailBuckets.some((bucket) => bucket.unpriced);
  const totalTokens =
    sum(principalBuckets, (bucket) => bucket.prompt_tokens) +
    sum(principalBuckets, (bucket) => bucket.completion_tokens);
  const hasUsage = dayBuckets.length > 0 || principalBuckets.length > 0 || modelBuckets.length > 0;

  const chartData = dayBuckets.map((bucket) => ({
    day: bucket.key,
    cost: Number(BigInt(bucket.cost_usd_nanos)) / 1_000_000_000,
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
          {row.unpriced ? 'Unknown' : formatUsdNanos(row.cost_usd_nanos)}
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
          {row.unpriced ? 'Unknown' : formatUsdNanos(row.cost_usd_nanos)}
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
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" onClick={() => setPriceModel('')}>
            Set model price
          </Button>
          <PeriodSelector period={period} contextQuery={contextQuery} />
        </div>
      </div>
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <SummaryTile label="Total spend" value={totalSpend} />
        <SummaryTile
          label="Guardrail overhead"
          value={
            hasUnpricedGuardrailUsage
              ? `${formatUsdNanos(guardrailCostNanos.toString())} + unknown`
              : formatUsdNanos(guardrailCostNanos.toString())
          }
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
  const [customModel, setCustomModel] = useState('');

  async function savePrice() {
    if (model === null) return;
    const resolvedModel = (model || customModel).trim();
    if (!resolvedModel) {
      toast.error('Model is required');
      return;
    }
    let payload: {
      input_per_million_minor: number;
      output_per_million_minor: number;
      input_per_million_usd_nanos: string;
      output_per_million_usd_nanos: string;
    };
    try {
      const input = dollarsToRate(inputPrice);
      const output = dollarsToRate(outputPrice);
      payload = {
        input_per_million_minor: input.minor,
        output_per_million_minor: output.minor,
        input_per_million_usd_nanos: input.nanos,
        output_per_million_usd_nanos: output.nanos,
      };
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Price is invalid');
      return;
    }
    setSaving(true);
    try {
      const response = await fetch(
        `/api/llm-pricing/${encodeURIComponent(resolvedModel)}${contextQuery}`,
        {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload),
        },
      );
      const text = await response.text();
      if (!response.ok) {
        throw new Error(safeError(text) ?? 'Unable to set model price');
      }
      toast.success(`Price set for ${resolvedModel}`);
      onOpenChange(false);
      setInputPrice('');
      setOutputPrice('');
      setCustomModel('');
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
            USD per 1M tokens
            {model ? (
              <>
                {' '}
                for <span className="font-mono">{model}</span>
              </>
            ) : null}
            . New calls are metered at this workspace price immediately.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3 md:grid-cols-2">
          {model === '' ? (
            <div className="grid gap-1.5 md:col-span-2">
              <Label htmlFor="set-price-model">Provider model</Label>
              <Input
                id="set-price-model"
                value={customModel}
                onChange={(event) => setCustomModel(event.target.value)}
                placeholder="deepseek-4-flash"
              />
            </div>
          ) : null}
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

function dollarsToRate(value: string): { minor: number; nanos: string } {
  const parsed = priceRateSchema.safeParse(value);
  if (!parsed.success) {
    throw new Error('Prices must be non-negative dollars with up to nine decimals');
  }
  const [dollars = '0', fraction = ''] = parsed.data.split('.');
  const nanos = BigInt(dollars) * 1_000_000_000n + BigInt(fraction.padEnd(9, '0'));
  return { minor: Number(nanos / 10_000_000n), nanos: nanos.toString() };
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
            href={`/usage${contextQuery}&period=${option}#usage-overview-title`}
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

function sumNanos(buckets: LlmUsageBucket[]): bigint {
  return buckets.reduce((total, bucket) => total + BigInt(bucket.cost_usd_nanos), 0n);
}
