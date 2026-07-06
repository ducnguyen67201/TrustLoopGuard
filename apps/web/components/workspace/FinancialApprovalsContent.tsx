'use client';

import { IconCheck, IconX } from '@tabler/icons-react';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { z } from 'zod';
import type { FinancialActionRecord, FinancialApprovalRequest } from '@trustloopguard/sdk';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { PageHeader } from '@/components/ui/page-header';
import {
  counterpartyLabel,
  currentContextQuery,
  FinancialStatusBadge,
  formatDateTime,
  formatMoney,
} from './financial-utils';

type FinancialApprovalsContentProps = {
  workspaceSlug: string;
  environmentId: string;
  approvals: FinancialApprovalRequest[];
  actions: FinancialActionRecord[];
};

export function FinancialApprovalsContent({
  workspaceSlug,
  environmentId,
  approvals,
  actions,
}: FinancialApprovalsContentProps) {
  const [approvalRows, setApprovalRows] = useState(approvals);
  const [actionRows, setActionRows] = useState(actions);
  const [busyActionIds, setBusyActionIds] = useState<string[]>([]);
  const actionById = useMemo(
    () => new Map(actionRows.map((action) => [action.id, action])),
    [actionRows],
  );
  const busySet = useMemo(() => new Set(busyActionIds), [busyActionIds]);
  const pendingRows = approvalRows.filter((approval) => approval.status === 'pending');
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);

  const columns: DataTableColumn<FinancialApprovalRequest>[] = [
    {
      id: 'action',
      header: 'Action',
      cell: (row) => {
        const action = actionById.get(row.action_id);
        return (
          <div className="grid min-w-0 gap-0.5">
            <span className="truncate text-sm font-medium text-foreground">
              {action?.action.kind ?? 'financial action'}
            </span>
            <span className="truncate font-mono text-xs text-muted-foreground">
              {row.action_id}
            </span>
          </div>
        );
      },
    },
    {
      id: 'status',
      header: 'Status',
      cell: (row) => {
        const action = actionById.get(row.action_id);
        return action ? <FinancialStatusBadge status={action.status} /> : row.status;
      },
    },
    {
      id: 'amount',
      header: 'Amount',
      align: 'right',
      cell: (row) => {
        const action = actionById.get(row.action_id);
        return action ? <span className="font-mono text-sm">{formatMoney(action)}</span> : '—';
      },
    },
    {
      id: 'counterparty',
      header: 'Counterparty',
      cell: (row) => {
        const action = actionById.get(row.action_id);
        return action ? counterpartyLabel(action) : '—';
      },
    },
    {
      id: 'reason',
      header: 'Reason',
      cell: (row) => <span className="text-sm text-muted-foreground">{row.reason}</span>,
    },
    {
      id: 'created',
      header: 'Created',
      cell: (row) => (
        <span className="text-sm text-muted-foreground">{formatDateTime(row.created_at)}</span>
      ),
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cell: (row) => {
        const busy = busySet.has(row.action_id);
        return (
          <div className="flex justify-end gap-1.5">
            <Button
              type="button"
              size="sm"
              disabled={busy}
              onClick={() => approveAndExecute(row.action_id)}
              aria-label={`Approve financial action ${row.action_id}`}
            >
              <IconCheck />
              Approve
            </Button>
            <Button
              type="button"
              size="sm"
              variant="outline"
              disabled={busy}
              onClick={() => deny(row.action_id)}
              aria-label={`Deny financial action ${row.action_id}`}
            >
              <IconX />
              Deny
            </Button>
          </div>
        );
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
        title="Approvals"
        description="Pending financial actions that need an approval decision before execution."
      />
      <Card>
        <CardHeader>
          <CardTitle>Queue</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={columns}
            rows={pendingRows}
            getRowKey={(row) => row.id}
            empty={
              <EmptyState
                title="No pending approvals"
                description="Held financial actions will appear here."
              />
            }
            caption="Financial approval queue"
          />
        </CardContent>
      </Card>
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
