import Link from 'next/link';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';

import { formatMinorUnits } from '../financial-utils';
import { SpendIntensityBar } from './SpendIntensityBar';
import { bySpendDescending, formatTokens, USAGE_CURRENCY } from './usage-utils';

type SpendByPrincipalCardProps = {
  principalBuckets: LlmUsageBucket[];
  /**
   * When set, each principal row links to this drill-in route with
   * `?principal=<key>` appended — the per-user focus affordance.
   */
  drillHrefBase?: string;
  /** Trim to the top N spenders (used by tighter embeds like the Financial page). */
  limit?: number;
  title?: string;
  description?: string;
  className?: string;
};

/**
 * THE hero widget: per-principal spend, heaviest first, with a solid magnitude
 * bar in the Spend column (same heat treatment as the home snapshot's mini bars)
 * so the budget-burner is obvious before you read a number. The column set is
 * shaped so a future `% of cap` slots in beside Spend without reflowing the table.
 */
export function SpendByPrincipalCard({
  principalBuckets,
  drillHrefBase,
  limit,
  title = 'Spend by principal',
  description = 'Who is burning the budget, heaviest first.',
  className,
}: SpendByPrincipalCardProps) {
  const sorted = bySpendDescending(principalBuckets);
  const rows = limit ? sorted.slice(0, limit) : sorted;
  const maxCostMinor = Number(rows[0]?.cost_minor ?? 0);
  const topKey = rows[0]?.key;
  const hiddenCount = limit ? Math.max(0, sorted.length - rows.length) : 0;

  const columns: DataTableColumn<LlmUsageBucket>[] = [
    {
      id: 'principal',
      header: 'Principal',
      cell: (row) => <PrincipalCell row={row} drillHrefBase={drillHrefBase} />,
    },
    {
      id: 'in',
      header: 'Tokens in',
      align: 'right',
      cellClassName: 'font-mono text-sm tabular-nums',
      cell: (row) => formatTokens(row.prompt_tokens),
    },
    {
      id: 'out',
      header: 'Tokens out',
      align: 'right',
      cellClassName: 'font-mono text-sm tabular-nums',
      cell: (row) => formatTokens(row.completion_tokens),
    },
    {
      id: 'calls',
      header: 'Calls',
      align: 'right',
      cellClassName: 'font-mono text-sm tabular-nums text-muted-foreground',
      cell: (row) => Number(row.calls).toLocaleString('en-US'),
    },
    {
      id: 'cost',
      header: 'Spend',
      align: 'right',
      // The heat treatment rides in the Spend cell so the visual weight lands on
      // the money column — where the eye should go first. A full-width magnitude
      // bar under the number reads as "how much" before the digits do.
      className: 'w-40',
      cell: (row) => (
        <SpendCell row={row} maxCostMinor={maxCostMinor} isTop={row.key === topKey} />
      ),
    },
  ];

  return (
    <Card className={className}>
      <CardHeader>
        <CardDescription>{description}</CardDescription>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-3">
        <DataTable
          columns={columns}
          rows={rows}
          getRowKey={(row) => row.key}
          caption="LLM usage by principal"
          empty={
            <EmptyState
              title="No principal usage in this period"
              description="Point an agent at the gateway — metered spend is attributed per principal here."
            />
          }
        />
        {hiddenCount > 0 && drillHrefBase ? (
          <Link
            href={drillHrefBase}
            className="text-xs font-medium text-primary hover:underline"
          >
            +{hiddenCount} more {hiddenCount === 1 ? 'principal' : 'principals'} — view all
          </Link>
        ) : null}
      </CardContent>
    </Card>
  );
}

function PrincipalCell({
  row,
  drillHrefBase,
}: {
  row: LlmUsageBucket;
  drillHrefBase: string | undefined;
}) {
  const label = <span className="truncate font-mono text-xs">{row.key}</span>;
  if (!drillHrefBase) return label;
  const separator = drillHrefBase.includes('?') ? '&' : '?';
  return (
    <Link
      href={`${drillHrefBase}${separator}principal=${encodeURIComponent(row.key)}`}
      className="truncate font-mono text-xs text-foreground hover:text-primary hover:underline"
      title={`Focus ${row.key}`}
    >
      {row.key}
    </Link>
  );
}

function SpendCell({
  row,
  maxCostMinor,
  isTop,
}: {
  row: LlmUsageBucket;
  maxCostMinor: number;
  isTop: boolean;
}) {
  return (
    <div className="grid justify-items-end gap-1.5">
      <span
        className={`font-mono text-sm tabular-nums ${
          isTop ? 'font-bold text-foreground' : 'font-medium text-foreground'
        }`}
      >
        {formatMinorUnits(row.cost_minor, USAGE_CURRENCY)}
      </span>
      <SpendIntensityBar bucket={row} maxCostMinor={maxCostMinor} />
    </div>
  );
}
