'use client';

import {
  IconCheck,
  IconClockHour4,
  IconFingerprint,
  IconHistory,
  IconShieldCheck,
  IconX,
} from '@tabler/icons-react';
import type { AuthorizationApproval, AuthorizationDomain, GrantMode } from '@trustloopguard/sdk';
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

type Props = {
  workspaceSlug: string;
  environmentId: string;
  approvals: AuthorizationApproval[];
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

export function AuthorizationApprovalsContent({ workspaceSlug, environmentId, approvals }: Props) {
  const [rows, setRows] = useState(approvals);
  const [domain, setDomain] = useState<AuthorizationDomain | 'all'>('all');
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
      cell: (row) => (
        <div className="grid gap-1">
          <div className="flex items-center gap-2">
            <Badge variant="outline">{row.envelope.domain}</Badge>
            <span className="font-medium">{row.envelope.capability}</span>
          </div>
          <span className="font-mono text-xs text-muted-foreground">{row.envelope.subject_id}</span>
        </div>
      ),
    },
    {
      id: 'principal',
      header: 'Principal',
      cell: (row) => (
        <div className="grid gap-1">
          <span>{row.envelope.principal_id}</span>
          <span className="font-mono text-xs text-muted-foreground">
            {shortHash(row.envelope_hash)}
          </span>
        </div>
      ),
    },
    {
      id: 'scope',
      header: 'Reviewed boundary',
      cell: (row) => (
        <div className="max-w-sm space-y-1">
          <p className="text-sm">{scopeSummary(row)}</p>
          <p className="font-mono text-xs text-muted-foreground">
            {row.envelope.exact_fingerprint}
          </p>
        </div>
      ),
    },
    {
      id: 'expires',
      header: 'Expires',
      cell: (row) => <TimeCell date={row.expires_at} prefix="in" />,
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cell: (row) => (
        <div className="flex justify-end gap-2">
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

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow="Unified authorization"
        title="Approvals"
        description="The only queue that can authorize or deny a waiting action across tools, content workflows, and finance."
      />
      <div className="grid gap-3 md:grid-cols-4">
        <MetricCard label="Pending" value={summary.pending} icon={<IconClockHour4 />} />
        <MetricCard label="Expiring soon" value={summary.expiringSoon} icon={<IconShieldCheck />} />
        <MetricCard label="Reusable scope" value={summary.scoped} icon={<IconFingerprint />} />
        <MetricCard label="History" value={summary.history} icon={<IconHistory />} />
      </div>
      <Tabs defaultValue="pending">
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <TabsList aria-label="Approval views">
            <TabsTrigger value="pending">Pending</TabsTrigger>
            <TabsTrigger value="history">History</TabsTrigger>
          </TabsList>
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
        </div>
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
              TrustLoopGuard re-evaluates current policy and domain state before issuing a
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
        if (row.envelope.proposed_scope) summary.scoped += 1;
      } else {
        summary.history += 1;
      }
      return summary;
    },
    { pending: 0, expiringSoon: 0, scoped: 0, history: 0 },
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
      <span className="text-xs text-muted-foreground">{formatDate(date)}</span>
    </div>
  );
}

function formatDate(date: string): string {
  return new Date(date).toLocaleString();
}

function shortHash(value: string): string {
  return value.length > 18 ? `${value.slice(0, 12)}...${value.slice(-6)}` : value;
}
