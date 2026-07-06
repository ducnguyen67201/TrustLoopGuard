'use client';

import { IconExternalLink, IconReceipt, IconSettings } from '@tabler/icons-react';
import Link from 'next/link';
import type {
  FinancialActionOutcome,
  FinancialActionRecord,
  GatewayProviderConnection,
} from '@trustloopguard/sdk';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { PageHeader } from '@/components/ui/page-header';
import type { FamilyPolicyRow } from '@/lib/server/dashboard-data';
import {
  counterpartyLabel,
  currentContextQuery,
  FinancialStatusBadge,
  formatDateTime,
  formatMoney,
  latestOutcome,
  OutcomeBadge,
  titleLabel,
} from './financial-utils';

type FinancialActionsContentProps = {
  workspaceSlug: string;
  environmentId: string;
  actions: FinancialActionRecord[];
  outcomesByActionId: Record<string, FinancialActionOutcome[]>;
  familyPolicies: FamilyPolicyRow[];
  providerConnections: GatewayProviderConnection[];
};

export function FinancialActionsContent({
  workspaceSlug,
  environmentId,
  actions,
  outcomesByActionId,
  familyPolicies,
  providerConnections,
}: FinancialActionsContentProps) {
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);
  const heldCount = actions.filter((action) => action.status === 'held').length;
  const executedCount = actions.filter((action) => action.status === 'executed').length;
  const failedCount = actions.filter(
    (action) => action.status === 'failed' || action.status === 'denied',
  ).length;
  const paymentProviders = providerConnections.filter((provider) => provider.kind === 'payment_http');
  const financialPolicies = familyPolicies.filter(
    (policy) => policy.family === 'financial' || policy.family === 'payment',
  );

  const columns: DataTableColumn<FinancialActionRecord>[] = [
    {
      id: 'status',
      header: 'Status',
      cell: (row) => <FinancialStatusBadge status={row.status} />,
    },
    {
      id: 'action',
      header: 'Action',
      cell: (row) => (
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate text-sm font-medium text-foreground">
            {titleLabel(row.action.kind)}
          </span>
          <span className="truncate font-mono text-xs text-muted-foreground">{row.id}</span>
        </div>
      ),
    },
    {
      id: 'amount',
      header: 'Amount',
      align: 'right',
      cell: (row) => <span className="font-mono text-sm tabular-nums">{formatMoney(row)}</span>,
    },
    {
      id: 'counterparty',
      header: 'Counterparty',
      cell: (row) => <span className="text-sm">{counterpartyLabel(row)}</span>,
    },
    {
      id: 'agent',
      header: 'Agent',
      cell: (row) => <span className="font-mono text-xs">{row.action.principal_id}</span>,
    },
    {
      id: 'outcome',
      header: 'Outcome',
      cell: (row) => <OutcomeBadge outcome={latestOutcome(outcomesByActionId, row.id)} />,
    },
    {
      id: 'created',
      header: 'Created',
      cell: (row) => <span className="text-sm text-muted-foreground">{formatDateTime(row.created_at)}</span>,
    },
    {
      id: 'receipt',
      header: '',
      align: 'right',
      cell: (row) =>
        row.status === 'executed' ? (
          <Button asChild variant="ghost" size="sm">
            <Link href={`/financial/receipts/${encodeURIComponent(row.id)}${contextQuery}`}>
              <IconReceipt />
              Receipt
            </Link>
          </Button>
        ) : null,
    },
  ];

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Financial authorization"
        title="Financial actions"
        description="Typed spend, refund, payout, and approval records from the Rust financial authorization service."
        actions={
          <Button asChild variant="outline">
            <Link href={`/financial/approvals${contextQuery}`}>
              <IconExternalLink />
              Approvals
            </Link>
          </Button>
        }
      />
      <div className="grid gap-3 md:grid-cols-3">
        <SummaryTile label="Held" value={heldCount} tone="held" />
        <SummaryTile label="Executed" value={executedCount} tone="executed" />
        <SummaryTile label="Denied or failed" value={failedCount} tone="failed" />
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Ledger</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={columns}
            rows={actions}
            getRowKey={(row) => row.id}
            empty={<EmptyState title="No financial actions" description="Actions will appear here after an agent submits a typed financial request." />}
            caption="Financial actions"
          />
        </CardContent>
      </Card>
      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Spending controls</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-2">
            {financialPolicies.length === 0 ? (
              <p className="text-sm text-muted-foreground">No financial or payment-family controls are enabled.</p>
            ) : (
              financialPolicies.map((policy) => (
                <div key={policy.id} className="flex flex-wrap items-center justify-between gap-2 rounded-md border px-3 py-2">
                  <div className="grid min-w-0 gap-0.5">
                    <span className="truncate text-sm font-medium">{policy.id}</span>
                    <span className="text-xs text-muted-foreground">{titleLabel(policy.family)}</span>
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    <Cap label="per action" value={policy.per_transaction_minor} />
                    <Cap label="daily" value={policy.daily_minor} />
                    <Cap label="monthly" value={policy.monthly_minor} />
                    <Cap label="hold" value={policy.hold_above_minor} />
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Provider setup</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-medium">{paymentProviders.length} payment provider{paymentProviders.length === 1 ? '' : 's'}</p>
                <p className="text-sm text-muted-foreground">Payment rails execute only through vaulted provider credentials.</p>
              </div>
              <Button asChild variant="outline">
                <Link href={`/gateway${contextQuery}`}>
                  <IconSettings />
                  Gateway
                </Link>
              </Button>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {paymentProviders.length === 0 ? (
                <Badge variant="escalate">No payment_http provider</Badge>
              ) : (
                paymentProviders.map((provider) => (
                  <Badge key={provider.id} variant="outline">
                    {provider.display_name}
                  </Badge>
                ))
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function SummaryTile({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone: 'held' | 'executed' | 'failed';
}) {
  const color =
    tone === 'executed'
      ? 'text-[var(--color-allow)]'
      : tone === 'held'
        ? 'text-[var(--color-escalate)]'
        : 'text-[var(--color-block)]';
  return (
    <div className="rounded-lg border bg-card px-4 py-3">
      <p className="text-xs uppercase text-muted-foreground">{label}</p>
      <p className={`mt-1 font-mono text-2xl font-semibold tabular-nums ${color}`}>{value}</p>
    </div>
  );
}

function Cap({ label, value }: { label: string; value: number | null | undefined }) {
  if (value == null) return null;
  return (
    <Badge variant="outline" className="font-mono text-xs tabular-nums">
      {label} {(value / 100).toLocaleString(undefined, { style: 'currency', currency: 'USD' })}
    </Badge>
  );
}
