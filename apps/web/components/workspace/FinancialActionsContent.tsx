'use client';

import { IconCheck, IconReceipt, IconSettings, IconX } from '@tabler/icons-react';
import Link from 'next/link';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { z } from 'zod';
import type {
  FinancialActionOutcome,
  FinancialActionRecord,
  FinancialApprovalRequest,
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
  approvals: FinancialApprovalRequest[];
  outcomesByActionId: Record<string, FinancialActionOutcome[]>;
  familyPolicies: FamilyPolicyRow[];
  providerConnections: GatewayProviderConnection[];
};

export function FinancialActionsContent({
  workspaceSlug,
  environmentId,
  actions,
  approvals,
  outcomesByActionId,
  familyPolicies,
  providerConnections,
}: FinancialActionsContentProps) {
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);
  const [actionRows, setActionRows] = useState(actions);
  const [approvalRows, setApprovalRows] = useState(approvals);
  const [busyActionIds, setBusyActionIds] = useState<string[]>([]);
  const busySet = useMemo(() => new Set(busyActionIds), [busyActionIds]);
  const pendingApprovalActionIds = useMemo(
    () =>
      new Set(
        approvalRows
          .filter((approval) => approval.status === 'pending')
          .map((approval) => approval.action_id),
      ),
    [approvalRows],
  );
  const approvalByActionId = useMemo(
    () => new Map(approvalRows.map((approval) => [approval.action_id, approval])),
    [approvalRows],
  );
  const heldCount = actionRows.filter((action) => action.status === 'held').length;
  const executedCount = actionRows.filter((action) => action.status === 'executed').length;
  const failedCount = actionRows.filter(
    (action) => action.status === 'failed' || action.status === 'denied',
  ).length;
  const paymentProviders = providerConnections.filter(
    (provider) => provider.kind === 'payment_http',
  );
  const financialPolicies = familyPolicies;

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
          <span className="truncate font-mono text-xs text-muted-foreground">
            {row.action.operation}
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
      id: 'reason',
      header: 'Reason',
      cell: (row) => (
        <ReasonCell
          action={row}
          outcome={latestOutcome(outcomesByActionId, row.id)}
          approval={approvalByActionId.get(row.id)}
        />
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
        const busy = busySet.has(row.id);
        if (pendingApprovalActionIds.has(row.id)) {
          return (
            <div className="flex justify-end gap-1.5">
              <Button
                type="button"
                size="sm"
                disabled={busy}
                onClick={() => approveAndExecute(row.id)}
                aria-label={`Approve financial action ${row.id}`}
              >
                <IconCheck />
                Approve
              </Button>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={busy}
                onClick={() => deny(row.id)}
                aria-label={`Deny financial action ${row.id}`}
              >
                <IconX />
                Deny
              </Button>
            </div>
          );
        }
        return row.status === 'executed' ? (
          <Button asChild variant="ghost" size="sm">
            <Link href={`/financial/receipts/${encodeURIComponent(row.id)}${contextQuery}`}>
              <IconReceipt />
              Receipt
            </Link>
          </Button>
        ) : null;
      },
    },
  ];

  async function approveAndExecute(actionId: string) {
    setBusyActionIds((prev) => [...prev, actionId]);
    try {
      const approved = await postAction(actionId, 'approve', contextQuery);
      setActionRows((prev) => upsertAction(prev, approved));
      setApprovalRows((prev) =>
        prev.map((approval) =>
          approval.action_id === actionId ? { ...approval, status: 'approved' } : approval,
        ),
      );
      const executed = await postAction(actionId, 'execute', contextQuery);
      setActionRows((prev) => upsertAction(prev, executed));
      toast.success(
        executed.status === 'executed' ? 'Action approved and executed' : 'Action approved',
      );
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Approval failed');
    } finally {
      setBusyActionIds((prev) => prev.filter((id) => id !== actionId));
    }
  }

  async function deny(actionId: string) {
    setBusyActionIds((prev) => [...prev, actionId]);
    try {
      const denied = await postAction(actionId, 'deny', contextQuery);
      setActionRows((prev) => upsertAction(prev, denied));
      setApprovalRows((prev) =>
        prev.map((approval) =>
          approval.action_id === actionId ? { ...approval, status: 'denied' } : approval,
        ),
      );
      toast.success('Action denied');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Deny failed');
    } finally {
      setBusyActionIds((prev) => prev.filter((id) => id !== actionId));
    }
  }

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Financial authorization"
        title="Financial actions"
        description="Typed spend, refund, payout, and approval records from the Rust financial authorization service."
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
            rows={actionRows}
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
      <div className="grid gap-4 lg:grid-cols-2">
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
                  Caps, holds, evidence checks, and approval thresholds live in the policy registry.
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

async function postAction(
  actionId: string,
  transition: 'approve' | 'deny' | 'execute',
  contextQuery: string,
) {
  const response = await fetch(
    `/api/financial/actions/${encodeURIComponent(actionId)}/${transition}${contextQuery}`,
    { method: 'POST' },
  );
  const text = await response.text();
  if (!response.ok) {
    throw new Error(safeError(text) ?? `Unable to ${transition} action`);
  }
  return financialActionRecordSchema.parse(JSON.parse(text)) as FinancialActionRecord;
}

const financialActionRecordSchema = z.looseObject({
  id: z.string(),
  workspace_id: z.string(),
  status: z.enum([
    'proposed',
    'authorized',
    'held',
    'executed',
    'denied',
    'failed',
    'reversed',
    'expired',
  ]),
  action: z.looseObject({
    kind: z.string(),
    operation: z.string(),
    principal_id: z.string(),
    amount: z.looseObject({
      amount_minor: z.union([z.number(), z.bigint()]),
      currency: z.string(),
    }),
  }),
  evidence: z.array(z.looseObject({})),
  created_at: z.string(),
  updated_at: z.string(),
});

function upsertAction(
  actions: FinancialActionRecord[],
  next: FinancialActionRecord,
): FinancialActionRecord[] {
  if (actions.some((action) => action.id === next.id)) {
    return actions.map((action) => (action.id === next.id ? next : action));
  }
  return [next, ...actions];
}

function safeError(text: string): string | null {
  try {
    const parsed = JSON.parse(text) as { error?: string; message?: string };
    return parsed.error ?? parsed.message ?? null;
  } catch {
    return text.trim() === '' ? null : text;
  }
}

function ReasonCell({
  action,
  outcome,
  approval,
}: {
  action: FinancialActionRecord;
  outcome: FinancialActionOutcome | undefined;
  approval: FinancialApprovalRequest | undefined;
}) {
  const reason = actionReason(action, outcome, approval);
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
  approval: FinancialApprovalRequest | undefined,
): { primary: string; secondary?: string } {
  if (action.status === 'held' && approval?.reason) {
    return { primary: approval.reason, secondary: 'Awaiting approval' };
  }

  if (action.status === 'denied' || action.status === 'failed') {
    const providerReason = stringMetadata(outcome?.metadata, 'reason');
    if (providerReason) {
      return { primary: cleanReason(providerReason), secondary: 'Execution failed' };
    }

    if (outcome?.provider_status && outcome.provider_status !== outcome.status) {
      return { primary: cleanReason(outcome.provider_status), secondary: 'Provider status' };
    }

    const evidenceFailure = firstFailedEvidenceReason(action);
    if (evidenceFailure) return { primary: evidenceFailure, secondary: 'Eligibility failed' };

    return { primary: 'No failure reason recorded', secondary: `Action ${action.status}` };
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
  ['mandate_valid', 'Mandate invalid'],
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
