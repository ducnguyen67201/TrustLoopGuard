import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';

import { formatMinorUnits } from '../financial-utils';
import { bySpendDescending, formatTokens, totalTokens, USAGE_CURRENCY } from './usage-utils';

type SpendByModelCardProps = {
  modelBuckets: LlmUsageBucket[];
  className?: string;
};

/** Per-model spend breakdown, heaviest model first. */
export function SpendByModelCard({ modelBuckets, className }: SpendByModelCardProps) {
  const rows = bySpendDescending(modelBuckets);

  const columns: DataTableColumn<LlmUsageBucket>[] = [
    {
      id: 'model',
      header: 'Model',
      cell: (row) => <span className="truncate font-mono text-xs">{row.key}</span>,
    },
    {
      id: 'calls',
      header: 'Calls',
      align: 'right',
      cellClassName: 'font-mono text-sm tabular-nums text-muted-foreground',
      cell: (row) => Number(row.calls).toLocaleString('en-US'),
    },
    {
      id: 'tokens',
      header: 'Tokens',
      align: 'right',
      cellClassName: 'font-mono text-sm tabular-nums',
      cell: (row) => formatTokens(totalTokens(row)),
    },
    {
      id: 'cost',
      header: 'Spend',
      align: 'right',
      cellClassName: 'font-mono text-sm font-medium tabular-nums',
      cell: (row) => formatMinorUnits(row.cost_minor, USAGE_CURRENCY),
    },
  ];

  return (
    <Card className={className}>
      <CardHeader>
        <CardDescription>Where the tokens go.</CardDescription>
        <CardTitle>Spend by model</CardTitle>
      </CardHeader>
      <CardContent>
        <DataTable
          columns={columns}
          rows={rows}
          getRowKey={(row) => row.key}
          caption="LLM usage by model"
          empty={<EmptyState title="No model usage in this period" />}
        />
      </CardContent>
    </Card>
  );
}
