'use client';

import { IconChecklist, IconReceipt, IconSettings } from '@tabler/icons-react';
import Link from 'next/link';
import { useState } from 'react';
import type {
  BudgetAlertConfig,
  BudgetAlertFiring,
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
import { BudgetAlertsCard } from '@/components/workspace/BudgetAlertsCard';
import { FinancialAuthorizationModel } from '@/components/workspace/FinancialAuthorizationModel';
import type { FamilyPolicyRow } from '@/lib/server/dashboard-data';
import {
  counterpartyLabel,
  currentContextQuery,
  FinancialAuthorizationBadge,
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
  budgetAlerts?: BudgetAlertConfig[];
  budgetAlertFirings?: BudgetAlertFiring[];
  focusActionId?: string | null;
};

export function FinancialActionsContent({
  workspaceSlug,
  environmentId,
  actions,
  outcomesByActionId,
  familyPolicies,
  providerConnections,
  budgetAlerts = [],
  budgetAlertFirings = [],
  focusActionId = null,
}: FinancialActionsContentProps) {
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);
  const [actionRows] = useState(actions);
  const heldCount = actionRows.filter(
    (action) => action.authorization_effect === 'require_approval',
  ).length;
  const executedCount = actionRows.filter(
    (action) => action.execution_status === 'succeeded',
  ).length;
  const failedCount = actionRows.filter(
    (action) => action.execution_status === 'failed' || action.authorization_effect === 'deny',
  ).length;
  const x402Actions = actionRows.filter((action) => action.action.rail === 'x402');
  const x402ReservedCount = x402Actions.filter(
    (action) =>
      (action.authorization_effect === 'permit' || action.authorization_effect === 'transform') &&
      action.execution_status === 'not_started',
  ).length;
  const paymentProviders = providerConnections.filter(
    (provider) => provider.kind === 'payment_http',
  );
  const financialPolicies = familyPolicies;
  const visibleActionRows =
    focusActionId === null
      ? actionRows
      : actionRows.filter((action) => action.id === focusActionId);

  const columns: DataTableColumn<FinancialActionRecord>[] = [
    {
      id: 'execution',
      header: 'Execution',
      cell: (row) => <FinancialStatusBadge status={row.execution_status} />,
    },
    {
      id: 'action',
      header: 'Action',
      cell: (row) => (
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate text-sm font-medium text-foreground">
            {titleLabel(row.action.kind)}
          </span>
          <span className="truncate font-mono text-xs text-muted-foreground">
            {row.action.operation}
          </span>
          <span>
            <Badge variant="outline">{titleLabel(row.action.rail)}</Badge>
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
      id: 'authorization',
      header: 'Authorization',
      cell: (row) => (
        <div className="grid gap-1">
          <FinancialAuthorizationBadge effect={row.authorization_effect} />
          <span className="text-xs text-muted-foreground">
            {titleLabel(row.authorization_status)}
          </span>
        </div>
      ),
    },
    {
      id: 'outcome',
      header: 'Outcome',
      cell: (row) => <OutcomeBadge outcome={latestOutcome(outcomesByActionId, row.id)} />,
    },
    {
      id: 'reason',
      header: 'Reason',
      cell: (row) => (
        <ReasonCell action={row} outcome={latestOutcome(outcomesByActionId, row.id)} />
      ),
    },
    {
      id: 'created',
      header: 'Created',
      cell: (row) => (
        <span className="text-sm text-muted-foreground">{formatDateTime(row.created_at)}</span>
      ),
    },
    {
      id: 'receipt',
      header: '',
      align: 'right',
      cell: (row) => {
        return (
          <div className="flex justify-end gap-1.5">
            {row.authorization_effect === 'require_approval' ? (
              <Button asChild variant="ghost" size="sm">
                <Link href={`/approvals${contextQuery}`}>
                  <IconChecklist />
                  Review
                </Link>
              </Button>
            ) : null}
            {row.execution_status === 'succeeded' ? (
              <Button asChild variant="ghost" size="sm">
                <Link href={`/financial/receipts/${encodeURIComponent(row.id)}${contextQuery}`}>
                  <IconReceipt />
                  Receipt
                </Link>
              </Button>
            ) : null}
          </div>
        );
      },
    },
  ];

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Financial authorization"
        title="Financial actions"
        description="Inspect financial authorization and execution outcomes. Human decisions happen only in Approvals."
      />
      <FinancialAuthorizationModel active="actions" contextQuery={contextQuery} />
      <div className="grid gap-3 md:grid-cols-4">
        <SummaryTile label="Held" value={heldCount} tone="held" />
        <SummaryTile label="Executed" value={executedCount} tone="executed" />
        <SummaryTile label="Denied or failed" value={failedCount} tone="failed" />
        <SummaryTile label="x402 reserved" value={x402ReservedCount} tone="x402" />
      </div>
      {focusActionId !== null ? (
        <div className="flex flex-col gap-1 rounded-lg border border-orange-300 bg-orange-50 p-3 text-sm dark:border-orange-900 dark:bg-orange-950/30">
          <strong>Reviewing this demo action</strong>
          <code className="break-all text-xs">{focusActionId}</code>
          <span className="text-muted-foreground">
            Open Approvals to authorize it; this ledger cannot mutate authorization state.
          </span>
        </div>
      ) : null}
      <Card>
        <CardHeader>
          <CardTitle>Ledger</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={columns}
            rows={visibleActionRows}
            getRowKey={(row) => row.id}
            empty={
              <EmptyState
                title="No financial actions"
                description="Actions will appear here after an agent submits a typed financial request."
              />
            }
            caption="Financial actions"
          />
        </CardContent>
      </Card>
      <div className="grid gap-4 lg:grid-cols-3">
        <Card>
          <CardHeader>
            <CardTitle>Policy controls</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-medium">
                  {financialPolicies.filter((policy) => policy.enabled !== false).length} active
                  financial {financialPolicies.length === 1 ? 'policy' : 'policies'}
                </p>
                <p className="text-sm text-muted-foreground">
                  Standing caps and grant requirements for agents live in the policy registry.
                </p>
              </div>
              <Button asChild variant="outline">
                <Link href={`/policies${contextQuery}`}>
                  <IconSettings />
                  Policies
                </Link>
              </Button>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {financialPolicies.length === 0 ? (
                <Badge variant="outline">No financial policies</Badge>
              ) : (
                financialPolicies.slice(0, 4).map((policy) => (
                  <Badge key={policy.id} variant="outline">
                    {policy.description ?? policy.id}
                  </Badge>
                ))
              )}
              {financialPolicies.length > 4 ? (
                <Badge variant="outline">+{financialPolicies.length - 4} more</Badge>
              ) : null}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Agentic payments</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-medium">
                  {x402Actions.length} x402 {x402Actions.length === 1 ? 'action' : 'actions'}
                </p>
                <p className="text-sm text-muted-foreground">
                  {x402ReservedCount} authorized,{' '}
                  {x402Actions.filter((action) => action.execution_status === 'succeeded').length}{' '}
                  committed
                </p>
              </div>
              <Badge variant={x402ReservedCount > 0 ? 'permit' : 'outline'}>x402</Badge>
            </div>
            <div className="flex flex-wrap gap-1.5">
              <Badge variant="outline">Authorize</Badge>
              <Badge variant="outline">Reserve</Badge>
              <Badge variant="outline">Commit</Badge>
              <Badge variant="outline">Rollback</Badge>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Provider setup</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-medium">
                  {paymentProviders.length} payment provider
                  {paymentProviders.length === 1 ? '' : 's'}
                </p>
                <p className="text-sm text-muted-foreground">
                  Payment rails execute only through vaulted provider credentials.
                </p>
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
                <Badge variant="require_approval">No payment_http provider</Badge>
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
      <BudgetAlertsCard
        contextQuery={contextQuery}
        configs={budgetAlerts}
        firings={budgetAlertFirings}
      />
    </div>
  );
}

function ReasonCell({
  action,
  outcome,
}: {
  action: FinancialActionRecord;
  outcome: FinancialActionOutcome | undefined;
}) {
  const reason = actionReason(action, outcome);
  return (
    <div className="grid min-w-40 gap-0.5">
      <span className="text-sm text-foreground">{reason.primary}</span>
      {reason.secondary ? (
        <span className="text-xs text-muted-foreground">{reason.secondary}</span>
      ) : null}
    </div>
  );
}

function actionReason(
  action: FinancialActionRecord,
  outcome: FinancialActionOutcome | undefined,
): { primary: string; secondary?: string } {
  if (action.authorization_effect === 'require_approval') {
    return { primary: 'Human authorization required', secondary: 'Open Approvals to decide' };
  }

  if (action.authorization_effect === 'deny' || action.execution_status === 'failed') {
    if (action.status_reason) {
      return {
        primary: cleanReason(action.status_reason),
        secondary:
          action.authorization_effect === 'deny' ? 'Authorization denied' : 'Execution failed',
      };
    }

    const providerReason = stringMetadata(outcome?.metadata, 'reason');
    if (providerReason) {
      return { primary: cleanReason(providerReason), secondary: 'Execution failed' };
    }

    if (outcome?.provider_status && outcome.provider_status !== outcome.status) {
      return { primary: cleanReason(outcome.provider_status), secondary: 'Provider status' };
    }

    const evidenceFailure = firstFailedEvidenceReason(action);
    if (evidenceFailure) return { primary: evidenceFailure, secondary: 'Eligibility failed' };

    return { primary: 'No failure reason recorded', secondary: 'Action could not proceed' };
  }

  const businessReason = stringMetadata(action.action.metadata, 'reason');
  if (businessReason) return { primary: cleanReason(businessReason), secondary: 'Request reason' };

  return { primary: action.action.memo ?? 'No reason recorded' };
}

const EVIDENCE_FAILURE_LABELS = [
  ['order_exists', 'Order not found'],
  ['payment_captured', 'Payment was not captured'],
  ['refund_window_open', 'Refund window closed'],
  ['amount_lte_refundable_balance', 'Amount exceeds refundable balance'],
  ['destination_is_original_payment_method', 'Not original payment method'],
  ['no_duplicate_refund', 'Duplicate refund'],
  ['invoice_matches_po', 'Invoice does not match PO'],
  ['vendor_approved', 'Vendor not approved'],
  ['grant_valid', 'Grant invalid'],
] as const;

function firstFailedEvidenceReason(action: FinancialActionRecord): string | null {
  for (const evidence of action.evidence) {
    const metadata = evidence.metadata;
    if (!isRecord(metadata)) continue;
    for (const [key, label] of EVIDENCE_FAILURE_LABELS) {
      if (metadata[key] === false) return label;
    }
  }
  return null;
}

function stringMetadata(metadata: Record<string, unknown> | null | undefined, key: string) {
  const value = metadata?.[key];
  return typeof value === 'string' && value.trim() !== '' ? value : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function cleanReason(reason: string): string {
  return titleLabel(reason.replaceAll(/[^a-zA-Z0-9]+/g, '_')).replaceAll('`', '');
}

function SummaryTile({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone: 'held' | 'executed' | 'failed' | 'x402';
}) {
  const color =
    tone === 'executed'
      ? 'text-[var(--color-permit)]'
      : tone === 'held'
        ? 'text-[var(--color-require-approval)]'
        : tone === 'x402'
          ? 'text-primary'
          : 'text-[var(--color-deny)]';
  return (
    <div className="rounded-lg border bg-card px-4 py-3">
      <p className="text-xs uppercase text-muted-foreground">{label}</p>
      <p className={`mt-1 font-mono text-2xl font-semibold tabular-nums ${color}`}>{value}</p>
    </div>
  );
}
