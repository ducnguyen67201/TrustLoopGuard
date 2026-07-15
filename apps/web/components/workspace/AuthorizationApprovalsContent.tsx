'use client';

import { IconCheck, IconFingerprint, IconX } from '@tabler/icons-react';
import type { AuthorizationApproval, AuthorizationDomain, GrantMode } from '@trustloopguard/sdk';
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

type Props = {
  workspaceSlug: string;
  environmentId: string;
  approvals: AuthorizationApproval[];
};

const decisionResponseSchema = z.object({
  approval: z.object({ id: z.string(), status: z.enum(['approved', 'denied']) }),
  grant: z.object({ id: z.string() }).nullable().optional(),
});
const domainFilterSchema = z.enum(['all', 'content', 'tool', 'financial']);

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
          row.id === result.approval.id ? { ...row, status: result.approval.status } : row,
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
      header: 'Intent',
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
    { id: 'principal', header: 'Principal', cell: (row) => row.envelope.principal_id },
    {
      id: 'scope',
      header: 'Reviewed scope',
      cell: (row) => (
        <pre className="max-w-md overflow-auto rounded-lg bg-muted p-3 text-xs">
          {JSON.stringify(row.envelope.proposed_scope ?? { exact: true }, null, 2)}
        </pre>
      ),
    },
    {
      id: 'expires',
      header: 'Expires',
      cell: (row) => new Date(row.expires_at).toLocaleString(),
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

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Unified authorization"
        title="Approvals"
        description="The only queue that can authorize or deny a waiting action across tools, content workflows, and finance."
      />
      <Card>
        <CardHeader className="flex-row items-center justify-between gap-4">
          <CardTitle>Pending decisions</CardTitle>
          <label className="flex items-center gap-2 text-sm">
            Domain
            <select
              className="rounded-md border bg-background px-3 py-2"
              value={domain}
              onChange={(event) => setDomain(domainFilterSchema.parse(event.target.value))}
            >
              <option value="all">All</option>
              <option value="tool">Tool</option>
              <option value="financial">Financial</option>
              <option value="content">Content</option>
            </select>
          </label>
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
