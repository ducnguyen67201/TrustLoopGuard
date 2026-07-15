'use client';

import { useMemo, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import type { FamilyPolicyRow } from '@/lib/server/dashboard-data';

import { FinancialPolicyCreateDialog } from './FinancialSpendingControlsCard';
import { formatMinorUnits } from './financial-utils';

export function UsageBudgetsCard({
  contextQuery,
  policies,
}: {
  contextQuery: string;
  policies: FamilyPolicyRow[];
}) {
  const [rows, setRows] = useState(policies.filter((policy) => policy.meter === 'llm_usage'));
  const [dialogOpen, setDialogOpen] = useState(false);
  const columns = useMemo<DataTableColumn<FamilyPolicyRow>[]>(
    () => [
      {
        id: 'policy',
        header: 'Budget',
        cell: (row) => (
          <div className="grid gap-0.5">
            <span className="font-medium">{row.description || row.id}</span>
            <span className="font-mono text-xs text-muted-foreground">{row.id}</span>
          </div>
        ),
      },
      {
        id: 'principal',
        header: 'Principal',
        cell: (row) => row.when?.agents?.[0] || 'Each principal',
      },
      {
        id: 'caps',
        header: 'Hard caps',
        cell: (row) => <span className="text-sm">{capLabel(row)}</span>,
      },
      {
        id: 'action',
        header: 'On breach',
        cell: (row) => <span className="capitalize">{row.on_breach ?? 'deny'}</span>,
      },
    ],
    [],
  );

  return (
    <>
      <Card id="budgets">
        <CardHeader className="flex flex-row items-start justify-between gap-4">
          <div className="grid gap-1">
            <CardTitle>LLM spending caps</CardTitle>
            <CardDescription>
              Bounded requests stop before the provider when their maximum would exceed a cap.
              Unbounded requests use actual cost and stop future calls after reaching it.
            </CardDescription>
          </div>
          <Button onClick={() => setDialogOpen(true)}>New LLM cap</Button>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={columns}
            rows={rows}
            getRowKey={(row) => row.id}
            caption="LLM spending caps"
            empty={
              <EmptyState
                title="No LLM spending cap"
                description="Gateway calls are still checked by policies, but provider spend is not capped."
                action={<Button onClick={() => setDialogOpen(true)}>Create a spending cap</Button>}
              />
            }
          />
        </CardContent>
      </Card>
      <FinancialPolicyCreateDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        contextQuery={contextQuery}
        meter="llm_usage"
        existingPolicyIds={rows.map((row) => row.id)}
        onCreated={(policy) =>
          setRows((current) => [...current.filter((row) => row.id !== policy.id), policy])
        }
      />
    </>
  );
}

function capLabel(policy: FamilyPolicyRow): string {
  const caps = [
    ['day', policy.daily_minor],
    ['week', policy.weekly_minor],
    ['month', policy.monthly_minor],
  ] as const;
  const labels = caps.flatMap(([window, amount]) =>
    amount === null || amount === undefined
      ? []
      : [`${formatMinorUnits(amount, 'USD')} / ${window}`],
  );
  return labels.join(' · ') || 'No cap';
}
