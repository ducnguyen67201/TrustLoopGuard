'use client';

import { IconBan, IconPlus } from '@tabler/icons-react';
import { useState, type FormEvent } from 'react';
import { toast } from 'sonner';
import { z } from 'zod';
import type { FinancialMandate } from '@trustloopguard/sdk';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import { Textarea } from '@/components/ui/textarea';
import {
  currentContextQuery,
  formatDateTime,
  MandateStatusBadge,
  safeError,
} from './financial-utils';

type FinancialMandatesContentProps = {
  workspaceSlug: string;
  environmentId: string;
  mandates: FinancialMandate[];
};

export function FinancialMandatesContent({
  workspaceSlug,
  environmentId,
  mandates,
}: FinancialMandatesContentProps) {
  const [rows, setRows] = useState(mandates);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [principalId, setPrincipalId] = useState('');
  const [scope, setScope] = useState('{\n  "action_kinds": ["refund"],\n  "currency": "USD"\n}');
  const [submitting, setSubmitting] = useState(false);
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);

  const columns: DataTableColumn<FinancialMandate>[] = [
    {
      id: 'principal',
      header: 'Principal',
      cell: (row) => (
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate font-mono text-xs text-foreground">{row.principal_id}</span>
          <span className="truncate font-mono text-xs text-muted-foreground">{row.id}</span>
        </div>
      ),
    },
    {
      id: 'status',
      header: 'Status',
      cell: (row) => <MandateStatusBadge mandate={row} />,
    },
    {
      id: 'scope',
      header: 'Scope',
      cell: (row) => (
        <code className="line-clamp-2 text-xs text-muted-foreground">
          {JSON.stringify(row.scope)}
        </code>
      ),
    },
    {
      id: 'version',
      header: 'Version',
      align: 'right',
      cell: (row) => <span className="font-mono text-sm tabular-nums">{row.version}</span>,
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
      cell: (row) =>
        row.status === 'active' ? (
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={busyId === row.id}
            onClick={() => confirmRevoke(row.id)}
            aria-label={`Revoke mandate ${row.id}`}
          >
            <IconBan />
            Revoke
          </Button>
        ) : null,
    },
  ];

  async function createMandate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const parsedScope = mandateScopeSchema.parse(JSON.parse(scope));
      const response = await fetch(`/api/financial/mandates${contextQuery}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          principal_id: principalId.trim(),
          scope: parsedScope,
          metadata: { source: 'dashboard' },
        }),
      });
      const text = await response.text();
      if (!response.ok) throw new Error(safeError(text) ?? 'Unable to create mandate');
      const mandate = JSON.parse(text) as FinancialMandate;
      setRows((prev) => [mandate, ...prev]);
      setPrincipalId('');
      toast.success('Mandate created');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Mandate create failed');
    } finally {
      setSubmitting(false);
    }
  }

  function confirmRevoke(id: string) {
    if (window.confirm('Revoke this financial mandate?')) {
      void revoke(id);
    }
  }

  async function revoke(id: string) {
    setBusyId(id);
    try {
      const response = await fetch(
        `/api/financial/mandates/${encodeURIComponent(id)}/revoke${contextQuery}`,
        { method: 'POST' },
      );
      const text = await response.text();
      if (!response.ok) throw new Error(safeError(text) ?? 'Unable to revoke mandate');
      const mandate = JSON.parse(text) as FinancialMandate;
      setRows((prev) => prev.map((row) => (row.id === id ? mandate : row)));
      toast.success('Mandate revoked');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Mandate revoke failed');
    } finally {
      setBusyId(null);
    }
  }

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Financial authorization"
        title="Mandates"
        description="Tenant-scoped authority records used to prove an agent is allowed to perform financial actions."
      />
      <Card>
        <CardHeader>
          <CardTitle>Mandates</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={columns}
            rows={rows}
            getRowKey={(row) => `${row.id}:${row.version}`}
            empty={
              <EmptyState
                title="No mandates"
                description="Create a mandate before requiring mandate proof in policy."
              />
            }
            caption="Financial mandates"
          />
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Create mandate</CardTitle>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={createMandate}
            className="grid gap-4 md:grid-cols-[minmax(0,18rem)_1fr_auto] md:items-end"
          >
            <div className="grid gap-2">
              <Label htmlFor="principal-id">Principal</Label>
              <Input
                id="principal-id"
                required
                value={principalId}
                onChange={(event) => setPrincipalId(event.target.value)}
                placeholder="refund-bot"
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="mandate-scope">Scope JSON</Label>
              <Textarea
                id="mandate-scope"
                required
                value={scope}
                onChange={(event) => setScope(event.target.value)}
                className="min-h-24 font-mono text-xs"
              />
            </div>
            <Button type="submit" disabled={submitting}>
              <IconPlus />
              Create
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

const mandateScopeSchema = z.looseObject({
  action_kinds: z.array(z.string()).min(1, 'Scope must include at least one action kind'),
  currency: z.string().trim().min(1, 'Scope currency is required'),
});
