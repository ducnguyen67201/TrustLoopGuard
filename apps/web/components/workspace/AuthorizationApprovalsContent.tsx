'use client';

import {
  IconActivity,
  IconCheck,
  IconClockHour4,
  IconFingerprint,
  IconHistory,
  IconShieldCheck,
  IconX,
} from '@tabler/icons-react';
import type {
  AuthorizationApproval,
  AuthorizationDomain,
  AuthorizationEffect,
  AuthorizationReceipt,
  GrantMode,
} from '@featherlane-ai/sdk';
import Link from 'next/link';
import type { ReactNode } from 'react';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { z } from 'zod';

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
import { PageHeader } from '@/components/ui/page-header';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';

import { FinancialAuthorizationBadge } from './financial-utils';

type Props = {
  workspaceSlug: string;
  environmentId: string;
  approvals: AuthorizationApproval[];
  receipts?: AuthorizationReceipt[];
};

const decisionResponseSchema = z.object({
  approval: z.object({
    id: z.string(),
    status: z.enum(['approved', 'denied']),
    decided_by: z.string().optional(),
    decided_at: z.string().optional(),
    decision_reason: z.string().optional(),
    grant_id: z.string().optional(),
  }),
  grant: z.object({ id: z.string() }).nullable().optional(),
});
const domainFilterSchema = z.enum(['all', 'content', 'tool', 'financial']);
const effectFilterSchema = z.enum([
  'all',
  'permit',
  'deny',
  'transform',
  'require_approval',
  'defer',
]);
const statusVariant: Record<
  AuthorizationApproval['status'],
  'outline' | 'permit' | 'deny' | 'defer'
> = {
  pending: 'outline',
  approved: 'permit',
  denied: 'deny',
  canceled: 'defer',
  expired: 'defer',
};

export function AuthorizationApprovalsContent({
  workspaceSlug,
  environmentId,
  approvals,
  receipts = [],
}: Props) {
  const [rows, setRows] = useState(approvals);
  const [domain, setDomain] = useState<AuthorizationDomain | 'all'>('all');
  const [effect, setEffect] = useState<AuthorizationEffect | 'all'>('all');
  const [selected, setSelected] = useState<AuthorizationApproval | null>(null);
  const [busy, setBusy] = useState(false);
  const query = `?workspace=${encodeURIComponent(workspaceSlug)}&environment=${encodeURIComponent(environmentId)}`;
  const visible = useMemo(
    () =>
      rows.filter(
        (row) => row.status === 'pending' && (domain === 'all' || row.envelope.domain === domain),
      ),
    [domain, rows],
  );
  const history = useMemo(
    () =>
      rows.filter(
        (row) => row.status !== 'pending' && (domain === 'all' || row.envelope.domain === domain),
      ),
    [domain, rows],
  );
  const activity = useMemo(
    () =>
      receipts.filter(
        (receipt) =>
          (domain === 'all' || receipt.domain === domain) &&
          (effect === 'all' || receipt.effect === effect),
      ),
    [domain, effect, receipts],
  );
  const summary = useMemo(() => buildSummary(rows), [rows]);

  async function decide(
    approval: AuthorizationApproval,
    decision: 'approve' | 'deny',
    mode: GrantMode,
  ) {
    setBusy(true);
    try {
      const response = await fetch(
        `/api/authorization/approvals/${encodeURIComponent(approval.id)}/decide${query}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            decision,
            mode,
            envelope_hash: approval.envelope_hash,
            ...(mode === 'scoped' ? { scope: approval.envelope.proposed_scope } : {}),
          }),
        },
      );
      const body = await response.text();
      if (!response.ok) throw new Error(readError(body) ?? `Decision failed (${response.status})`);
      const result = decisionResponseSchema.parse(JSON.parse(body));
      setRows((current) =>
        current.map((row) =>
          row.id === result.approval.id ? applyDecisionResult(row, result) : row,
        ),
      );
      setSelected(null);
      toast.success(decision === 'approve' ? 'Authorization granted' : 'Intent denied', {
        description:
          decision === 'approve'
            ? 'Current policies and live domain checks still run before execution.'
            : undefined,
      });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Decision failed');
    } finally {
      setBusy(false);
    }
  }

  const columns: DataTableColumn<AuthorizationApproval>[] = [
    {
      id: 'subject',
      header: 'Request',
      cellClassName: 'min-w-64 max-w-80',
      cell: (row) => (
        <div className="grid min-w-0 gap-1">
          <div className="flex items-center gap-2">
            <Badge variant="outline">{row.envelope.domain}</Badge>
            <span className="truncate font-medium">{row.envelope.capability}</span>
          </div>
          <span className="truncate font-mono text-xs text-muted-foreground">
            {row.envelope.subject_id}
          </span>
        </div>
      ),
    },
    {
      id: 'principal',
      header: 'Principal',
      cellClassName: 'w-52 max-w-52',
      cell: (row) => (
        <div className="grid min-w-0 gap-1">
          <span className="truncate">{row.envelope.principal_id}</span>
          <span className="font-mono text-xs text-muted-foreground">
            {shortHash(row.envelope_hash)}
          </span>
        </div>
      ),
    },
    {
      id: 'scope',
      header: 'Reviewed boundary',
      cellClassName: 'min-w-72 max-w-96',
      cell: (row) => (
        <div className="grid min-w-0 gap-1">
          <p className="truncate text-sm">{scopeSummary(row)}</p>
          <p className="truncate font-mono text-xs text-muted-foreground">
            {row.envelope.exact_fingerprint}
          </p>
        </div>
      ),
    },
    {
      id: 'expires',
      header: 'Expires',
      cellClassName: 'w-32 max-w-32',
      cell: (row) => <TimeCell date={row.expires_at} prefix="in" />,
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cellClassName: 'w-52 max-w-52',
      cell: (row) => (
        <div className="flex justify-end gap-2 whitespace-nowrap">
          <Button size="sm" onClick={() => setSelected(row)}>
            <IconCheck /> Review
          </Button>
          <Button size="sm" variant="outline" onClick={() => decide(row, 'deny', 'exact_once')}>
            <IconX /> Deny
          </Button>
        </div>
      ),
    },
  ];
  const historyColumns: DataTableColumn<AuthorizationApproval>[] = [
    {
      id: 'status',
      header: 'Decision',
      cell: (row) => (
        <div className="grid gap-1">
          <Badge variant={statusVariant[row.status]}>{row.status.replaceAll('_', ' ')}</Badge>
          <span className="text-xs text-muted-foreground">
            {row.decided_at ? formatDate(row.decided_at) : formatDate(row.updated_at)}
          </span>
        </div>
      ),
    },
    {
      id: 'subject',
      header: 'Request',
      cell: (row) => (
        <div className="grid gap-1">
          <span className="font-medium">{row.envelope.capability}</span>
          <span className="font-mono text-xs text-muted-foreground">{row.envelope.subject_id}</span>
        </div>
      ),
    },
    {
      id: 'principal',
      header: 'Principal',
      cell: (row) => row.envelope.principal_id,
    },
    {
      id: 'authority',
      header: 'Authority',
      cell: (row) => (
        <div className="grid gap-1">
          <span>{row.grant_id ? 'Grant minted' : 'No grant'}</span>
          <span className="font-mono text-xs text-muted-foreground">
            {row.grant_id ? shortHash(row.grant_id) : shortHash(row.envelope_hash)}
          </span>
        </div>
      ),
    },
    {
      id: 'scope',
      header: 'Reviewed boundary',
      cell: (row) => scopeSummary(row),
    },
  ];
  const activityColumns: DataTableColumn<AuthorizationReceipt>[] = [
    {
      id: 'time',
      header: 'Time',
      cellClassName: 'w-44 max-w-44',
      cell: (row) => (
        <time dateTime={row.created_at} className="text-xs text-muted-foreground">
          {formatDate(row.created_at)}
        </time>
      ),
    },
    {
      id: 'outcome',
      header: 'Outcome',
      cellClassName: 'w-36 max-w-36',
      cell: (row) => <FinancialAuthorizationBadge effect={row.effect} />,
    },
    {
      id: 'domain',
      header: 'Domain',
      cell: (row) => <Badge variant="outline">{row.domain}</Badge>,
    },
    {
      id: 'principal',
      header: 'Principal',
      cellClassName: 'min-w-44 max-w-56',
      cell: (row) => (
        <span className="block truncate" title={row.principal_id ?? undefined}>
          {row.principal_id ?? 'Legacy / unknown'}
        </span>
      ),
    },
    {
      id: 'operation',
      header: 'Operation',
      cellClassName: 'min-w-56 max-w-72',
      cell: (row) => (
        <div className="grid gap-1">
          <span className="truncate font-mono text-xs" title={row.operation ?? undefined}>
            {row.operation ?? '—'}
          </span>
          {row.run_id ? (
            <Link
              className="text-xs text-primary hover:underline"
              href={`/runs/${encodeURIComponent(row.run_id)}${query}`}
            >
              View run
            </Link>
          ) : null}
        </div>
      ),
    },
    {
      id: 'reason',
      header: 'Reason',
      cellClassName: 'min-w-64 max-w-md',
      cell: (row) => (
        <div className="grid gap-1">
          <span className="line-clamp-2 text-sm">{row.reason}</span>
          <Link
            className="text-xs text-primary hover:underline"
            href={`/authorization/receipts/${encodeURIComponent(row.id)}${query}`}
          >
            View receipt
          </Link>
        </div>
      ),
    },
  ];

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow="Unified authorization"
        title="Authorization"
        description="Review policy outcomes, act on waiting approvals, and inspect authorization history across tools, content workflows, and finance."
      />
      <div className="grid gap-3 md:grid-cols-4">
        <MetricCard label="Activity" value={receipts.length} icon={<IconActivity />} />
        <MetricCard label="Pending" value={summary.pending} icon={<IconClockHour4 />} />
        <MetricCard label="Expiring soon" value={summary.expiringSoon} icon={<IconShieldCheck />} />
        <MetricCard label="History" value={summary.history} icon={<IconHistory />} />
      </div>
      <Tabs defaultValue="activity">
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <TabsList aria-label="Authorization views">
            <TabsTrigger value="activity">Activity</TabsTrigger>
            <TabsTrigger value="pending">Needs approval</TabsTrigger>
            <TabsTrigger value="history">Approval history</TabsTrigger>
          </TabsList>
          <div className="flex flex-wrap gap-3">
            <label className="flex w-fit items-center gap-2 text-sm">
              Domain
              <select
                className="h-9 rounded-md border border-input bg-background px-3 text-sm shadow-xs"
                value={domain}
                onChange={(event) => setDomain(domainFilterSchema.parse(event.target.value))}
              >
                <option value="all">All</option>
                <option value="tool">Tool</option>
                <option value="financial">Financial</option>
                <option value="content">Content</option>
              </select>
            </label>
            <label className="flex w-fit items-center gap-2 text-sm">
              Outcome
              <select
                className="h-9 rounded-md border border-input bg-background px-3 text-sm shadow-xs"
                value={effect}
                onChange={(event) => setEffect(effectFilterSchema.parse(event.target.value))}
              >
                <option value="all">All</option>
                <option value="permit">Permit</option>
                <option value="deny">Deny</option>
                <option value="defer">Defer</option>
                <option value="require_approval">Require approval</option>
                <option value="transform">Transform</option>
              </select>
            </label>
          </div>
        </div>
        <TabsContent value="activity">
          <Card>
            <CardHeader>
              <CardTitle>Authorization activity</CardTitle>
              <p className="text-sm text-muted-foreground">
                Policy and authority outcomes. A permit is an evaluated outcome, not a human
                approval.
              </p>
            </CardHeader>
            <CardContent>
              <DataTable
                columns={activityColumns}
                rows={activity}
                getRowKey={(row) => row.id}
                caption="Authorization receipt activity"
                empty={
                  <EmptyState
                    title="No authorization activity"
                    description="Policy and authority receipts will appear here."
                  />
                }
              />
            </CardContent>
          </Card>
        </TabsContent>
        <TabsContent value="pending">
          <Card>
            <CardHeader className="flex-row items-center justify-between gap-4">
              <div className="grid gap-1">
                <CardTitle>Pending decisions</CardTitle>
                <p className="text-sm text-muted-foreground">
                  Immutable envelopes waiting for exact or bounded authority.
                </p>
              </div>
            </CardHeader>
            <CardContent>
              <DataTable
                columns={columns}
                rows={visible}
                getRowKey={(row) => row.id}
                caption="Unified authorization approval queue"
                empty={
                  <EmptyState
                    title="No pending approvals"
                    description="Waiting intents will appear here."
                  />
                }
              />
            </CardContent>
          </Card>
        </TabsContent>
        <TabsContent value="history">
          <Card>
            <CardHeader>
              <CardTitle>Approval history</CardTitle>
              <p className="text-sm text-muted-foreground">
                Recent resolved approvals from the Rust authorization ledger.
              </p>
            </CardHeader>
            <CardContent>
              <DataTable
                columns={historyColumns}
                rows={history}
                getRowKey={(row) => row.id}
                caption="Unified authorization approval history"
                empty={
                  <EmptyState
                    title="No approval history"
                    description="Approved, denied, canceled, and expired requests will appear here."
                  />
                }
              />
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
      {selected ? (
        <Dialog open onOpenChange={(open) => !open && !busy && setSelected(null)}>
          <DialogContent className="max-w-2xl">
            <DialogHeader>
              <DialogTitle>Review authorization</DialogTitle>
              <DialogDescription>
                Approval supplies authority; it never overrides a hard policy, missing evidence,
                eligibility, or a live budget check.
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-3 rounded-lg border bg-muted/30 p-4 text-sm">
              <Detail label="Capability" value={selected.envelope.capability} />
              <Detail label="Principal" value={selected.envelope.principal_id} />
              <Detail
                label="Subject fingerprint"
                value={selected.envelope.exact_fingerprint}
                mono
              />
              <Detail label="Envelope hash" value={selected.envelope_hash} mono />
            </div>
            <p className="text-sm text-muted-foreground">
              Featherlane AI re-evaluates current policy and domain state before issuing a
              one-attempt execution lease.
            </p>
            <DialogFooter>
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => decide(selected, 'deny', 'exact_once')}
              >
                <IconX /> Deny
              </Button>
              <Button disabled={busy} onClick={() => decide(selected, 'approve', 'exact_once')}>
                <IconCheck /> Approve this action
              </Button>
              {selected.envelope.proposed_scope ? (
                <Button disabled={busy} onClick={() => decide(selected, 'approve', 'scoped')}>
                  <IconFingerprint /> Approve matching actions
                </Button>
              ) : null}
            </DialogFooter>
          </DialogContent>
        </Dialog>
      ) : null}
    </div>
  );
}

function MetricCard({ label, value, icon }: { label: string; value: number; icon: ReactNode }) {
  return (
    <div className="grid gap-2 rounded-lg border bg-card p-4 shadow-xs">
      <div className="flex items-center justify-between gap-3 text-muted-foreground">
        <span className="text-xs font-medium uppercase tracking-label">{label}</span>
        <span className="[&_svg]:size-4">{icon}</span>
      </div>
      <span className="font-data text-2xl font-semibold">{value}</span>
    </div>
  );
}

function Detail({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="grid gap-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={mono ? 'break-all font-mono text-xs' : 'font-medium'}>{value}</span>
    </div>
  );
}

function readError(body: string): string | undefined {
  try {
    return z.object({ message: z.string() }).parse(JSON.parse(body)).message;
  } catch {
    return undefined;
  }
}

function buildSummary(rows: AuthorizationApproval[]) {
  const now = Date.now();
  return rows.reduce(
    (summary, row) => {
      if (row.status === 'pending') {
        summary.pending += 1;
        if (new Date(row.expires_at).getTime() - now <= 30 * 60 * 1000) {
          summary.expiringSoon += 1;
        }
      } else {
        summary.history += 1;
      }
      return summary;
    },
    { pending: 0, expiringSoon: 0, history: 0 },
  );
}

function applyDecisionResult(
  row: AuthorizationApproval,
  result: z.infer<typeof decisionResponseSchema>,
): AuthorizationApproval {
  const updated: AuthorizationApproval = {
    ...row,
    status: result.approval.status,
  };
  if (result.approval.decided_by) updated.decided_by = result.approval.decided_by;
  if (result.approval.decided_at) {
    updated.decided_at = result.approval.decided_at;
    updated.updated_at = result.approval.decided_at;
  }
  if (result.approval.decision_reason) {
    updated.decision_reason = result.approval.decision_reason;
  }
  const grantId = result.approval.grant_id ?? result.grant?.id;
  if (grantId) updated.grant_id = grantId;
  return updated;
}

function scopeSummary(row: AuthorizationApproval): string {
  const scope = row.envelope.proposed_scope;
  if (!scope) return 'Exact action only';
  if (scope.scope_type === 'action') {
    const operations = scope.scope.operations.join(', ');
    const destinations = scope.scope.allowed_destinations.join(', ');
    return destinations ? `${operations} to ${destinations}` : operations;
  }
  if (scope.scope_type === 'financial') {
    const operation = scope.scope.operation ?? scope.scope.action_kinds.join(', ');
    const ceiling =
      scope.scope.currency && scope.scope.maximum_amount_minor
        ? ` ${scope.scope.currency} up to ${scope.scope.maximum_amount_minor}`
        : '';
    return `${operation}${ceiling}`;
  }
  return 'Scoped approval';
}

function TimeCell({ date, prefix }: { date: string; prefix?: string }) {
  const expiry = new Date(date).getTime();
  const minutes = Math.max(0, Math.round((expiry - Date.now()) / 60000));
  const urgent = minutes <= 30;
  return (
    <div className="grid gap-1">
      <span className={cn('font-data text-sm', urgent ? 'text-destructive' : undefined)}>
        {prefix ? `${prefix} ` : null}
        {minutes < 60 ? `${minutes}m` : `${Math.round(minutes / 60)}h`}
      </span>
      <span className="truncate text-xs text-muted-foreground">{formatShortDate(date)}</span>
    </div>
  );
}

function formatDate(date: string): string {
  return new Date(date).toLocaleString();
}

function formatShortDate(date: string): string {
  return new Date(date).toLocaleString(undefined, {
    month: 'numeric',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
}

function shortHash(value: string): string {
  return value.length > 18 ? `${value.slice(0, 12)}...${value.slice(-6)}` : value;
}
