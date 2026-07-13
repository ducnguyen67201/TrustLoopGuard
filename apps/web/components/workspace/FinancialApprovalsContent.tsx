'use client';

import { IconCheck, IconFingerprint, IconX } from '@tabler/icons-react';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { z } from 'zod';
import type {
  ApproveMatchingFinancialActionsResponse,
  FinancialActionRecord,
  FinancialApprovalEnvelope,
  FinancialApprovalRequest,
} from '@trustloopguard/sdk';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import {
  counterpartyLabel,
  currentContextQuery,
  FinancialStatusBadge,
  formatDateTime,
  formatMoney,
  safeError,
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
  const [reuseActionId, setReuseActionId] = useState<string | null>(null);
  const [envelope, setEnvelope] = useState<FinancialApprovalEnvelope | null>(null);
  const [loadingEnvelope, setLoadingEnvelope] = useState(false);
  const [maxAmount, setMaxAmount] = useState('');
  const [expiresAt, setExpiresAt] = useState(defaultExpiryInput());
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
              aria-label={`Approve only financial action ${row.action_id}`}
            >
              <IconCheck />
              Approve this one
            </Button>
            <Button
              type="button"
              size="sm"
              variant="outline"
              disabled={busy}
              onClick={() => openReusableApproval(row.action_id)}
              aria-label={`Approve matching financial actions for ${row.action_id}`}
            >
              <IconFingerprint />
              Approve matching
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

  async function openReusableApproval(actionId: string) {
    setReuseActionId(actionId);
    setEnvelope(null);
    setLoadingEnvelope(true);
    setExpiresAt(defaultExpiryInput());
    try {
      const response = await fetch(
        `/api/financial/actions/${encodeURIComponent(actionId)}/approval-envelope${contextQuery}`,
      );
      const text = await response.text();
      if (!response.ok) {
        throw new Error(safeError(text) ?? 'Unable to load approval fingerprint');
      }
      const nextEnvelope = financialApprovalEnvelopeSchema.parse(
        JSON.parse(text),
      ) as FinancialApprovalEnvelope;
      setEnvelope(nextEnvelope);
      setMaxAmount(minorUnitsInput(nextEnvelope.current_amount_minor));
    } catch (error) {
      setReuseActionId(null);
      toast.error(error instanceof Error ? error.message : 'Unable to load approval fingerprint');
    } finally {
      setLoadingEnvelope(false);
    }
  }

  async function approveMatchingAndExecute() {
    if (!reuseActionId || !envelope) return;
    const actionId = reuseActionId;
    let maxAmountMinor: number;
    try {
      maxAmountMinor = parseMinorUnits(maxAmount);
      if (maxAmountMinor < Number(envelope.current_amount_minor)) {
        throw new Error('Maximum must cover this action');
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Maximum is invalid');
      return;
    }
    const expiry = new Date(expiresAt);
    if (Number.isNaN(expiry.getTime()) || expiry.getTime() <= Date.now()) {
      toast.error('Expiry must be in the future');
      return;
    }

    setBusyActionIds((prev) => [...prev, actionId]);
    try {
      const response = await fetch(
        `/api/financial/actions/${encodeURIComponent(actionId)}/approve-matching${contextQuery}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            action_fingerprint: envelope.action_fingerprint,
            max_amount_minor: maxAmountMinor,
            expires_at: expiry.toISOString(),
          }),
        },
      );
      const text = await response.text();
      if (!response.ok) {
        throw new Error(safeError(text) ?? 'Unable to activate reusable approval');
      }
      const result = approveMatchingResponseSchema.parse(
        JSON.parse(text),
      ) as ApproveMatchingFinancialActionsResponse;
      setActionRows((prev) => upsertAction(prev, result.action));
      setApprovalRows((prev) =>
        prev.map((approval) =>
          approval.action_id === actionId ? { ...approval, status: 'approved' } : approval,
        ),
      );
      setReuseActionId(null);
      toast.success('Mandate active', {
        description: 'Matching actions can now reuse this approval until it expires.',
      });
      try {
        const executed = await postAction(actionId, 'execute', contextQuery);
        setActionRows((prev) => upsertAction(prev, executed));
      } catch (error) {
        toast.error('Mandate is active, but the current action did not execute', {
          description: error instanceof Error ? error.message : undefined,
        });
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Reusable approval failed');
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
      <Dialog
        open={reuseActionId !== null}
        onOpenChange={(open) => {
          if (!open && !(reuseActionId && busySet.has(reuseActionId))) setReuseActionId(null);
        }}
      >
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Approve matching actions</DialogTitle>
            <DialogDescription>
              Reuse this human approval for the same bounded action shape. TrustLoopGuard still
              checks every future action before money moves.
            </DialogDescription>
          </DialogHeader>
          {loadingEnvelope || !envelope ? (
            <p className="text-sm text-muted-foreground">Computing action fingerprint…</p>
          ) : (
            <div className="grid gap-4">
              <div className="grid gap-3 rounded-lg border bg-muted/30 p-4 sm:grid-cols-2">
                <ApprovalDetail label="Principal" value={envelope.principal_id} mono />
                <ApprovalDetail
                  label="Action"
                  value={`${envelope.action_kind} · ${envelope.operation}`}
                />
                <ApprovalDetail label="Rail" value={envelope.rail} />
                <ApprovalDetail
                  label="Counterparty"
                  value={envelope.counterparty_id ?? 'No counterparty'}
                  mono
                />
              </div>
              <div className="grid gap-2">
                <div className="flex items-center justify-between gap-2">
                  <Label>Action fingerprint</Label>
                  <Badge variant="outline">v{envelope.fingerprint_version}</Badge>
                </div>
                <code className="break-all rounded-md border bg-muted px-3 py-2 text-xs">
                  {envelope.action_fingerprint}
                </code>
                <p className="text-xs text-muted-foreground">
                  Approval is bound to this action version. Amount is controlled separately by the
                  maximum below.
                </p>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="grid gap-2">
                  <Label htmlFor="reuse-max-amount">Maximum per action ({envelope.currency})</Label>
                  <Input
                    id="reuse-max-amount"
                    inputMode="decimal"
                    value={maxAmount}
                    onChange={(event) => setMaxAmount(event.target.value)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="reuse-expires-at">Expires at</Label>
                  <Input
                    id="reuse-expires-at"
                    type="datetime-local"
                    value={expiresAt}
                    onChange={(event) => setExpiresAt(event.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">Maximum reusable window: 30 days.</p>
                </div>
              </div>
              <p className="text-sm text-muted-foreground">
                Changes to principal, action kind, operation, rail, currency, counterparty, or x402
                destination require a new approval.
              </p>
              <div className="grid gap-2 rounded-lg border border-primary/20 bg-primary/5 p-4 text-sm">
                <p className="font-medium">Only the matching human-review step is reused.</p>
                <p className="text-muted-foreground">
                  Mandate status, hard policies, eligibility evidence, and the live available budget
                  are checked again for every action. A matching fingerprint never reserves or
                  guarantees funds in advance.
                </p>
              </div>
            </div>
          )}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              disabled={reuseActionId ? busySet.has(reuseActionId) : false}
              onClick={() => setReuseActionId(null)}
            >
              Cancel
            </Button>
            <Button
              type="button"
              disabled={!envelope || (reuseActionId ? busySet.has(reuseActionId) : false)}
              onClick={approveMatchingAndExecute}
            >
              <IconFingerprint />
              Approve once and reuse
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function ApprovalDetail({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="grid min-w-0 gap-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={mono ? 'truncate font-mono text-xs' : 'truncate text-sm font-medium'}>
        {value}
      </span>
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
  status_reason: z.string().optional().nullable(),
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

const financialApprovalEnvelopeSchema = z.looseObject({
  action_id: z.string(),
  action_fingerprint: z.string().startsWith('sha256:v'),
  fingerprint_version: z.number().int().positive(),
  principal_id: z.string(),
  action_kind: z.string(),
  operation: z.string(),
  rail: z.string(),
  currency: z.string(),
  counterparty_id: z.string().optional().nullable(),
  current_amount_minor: z.union([z.number(), z.bigint()]),
  recommended_max_amount_minor: z.union([z.number(), z.bigint()]),
});

const approveMatchingResponseSchema = z.looseObject({
  action: financialActionRecordSchema,
  mandate: z.looseObject({ id: z.string(), version: z.number(), status: z.string() }),
  approval_envelope: financialApprovalEnvelopeSchema,
});

function defaultExpiryInput(): string {
  const date = new Date(Date.now() + 24 * 60 * 60 * 1000);
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

function minorUnitsInput(amount: number | bigint): string {
  const minor = BigInt(amount);
  return `${minor / 100n}.${(minor % 100n).toString().padStart(2, '0')}`;
}

function parseMinorUnits(value: string): number {
  const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(value.trim());
  if (!match) throw new Error('Enter an amount with up to two decimal places');
  const major = match[1];
  if (!major) throw new Error('Enter a valid amount');
  const minor = BigInt(major) * 100n + BigInt((match[2] ?? '').padEnd(2, '0'));
  if (minor > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('Maximum is too large');
  return Number(minor);
}

function upsertAction(
  actions: FinancialActionRecord[],
  next: FinancialActionRecord,
): FinancialActionRecord[] {
  if (actions.some((action) => action.id === next.id)) {
    return actions.map((action) => (action.id === next.id ? next : action));
  }
  return [next, ...actions];
}
