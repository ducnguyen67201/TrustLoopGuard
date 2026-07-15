'use client';

import { IconBan, IconPlus } from '@tabler/icons-react';
import type { AuthorizationGrant } from '@trustloopguard/sdk';
import { useRouter } from 'next/navigation';
import { useMemo, useState, type ReactNode } from 'react';
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
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';

type Props = {
  workspaceSlug: string;
  environmentId: string;
  grants: AuthorizationGrant[];
};

const revokedGrantSchema = z.object({ id: z.string(), status: z.literal('revoked') });
const commonGrantFields = {
  principal: z.string().trim().min(1),
  capability: z
    .string()
    .trim()
    .regex(/^[a-z][a-z0-9_-]*:[^\s]+$/),
  requirementIds: z.string().trim().min(1),
  operation: z.string().trim().min(1),
  maxUses: z
    .string()
    .regex(/^$|^\d+$/)
    .refine((value) => value === '' || Number(value) > 0, 'Maximum uses must be positive'),
  expiresAt: z.string(),
};
const grantFormSchema = z.discriminatedUnion('domain', [
  z.object({
    domain: z.literal('tool'),
    ...commonGrantFields,
    sideEffect: z.enum([
      'none',
      'read',
      'external_communication',
      'file_write',
      'shell_exec',
      'network_call',
      'db_mutation',
      'api_mutation',
      'memory_write',
      'publish',
    ]),
    serverId: z.string(),
    toolName: z.string(),
    schemaHash: z.string(),
    parameters: z.string(),
  }),
  z.object({
    domain: z.literal('financial'),
    ...commonGrantFields,
    actionKind: z.enum([
      'payment',
      'refund',
      'payout',
      'invoice_approval',
      'purchase',
      'treasury_transfer',
      'consent',
      'other',
    ]),
    rail: z.enum(['payment_http', 'x402', 'card', 'ach', 'wire', 'internal', 'other']),
    currency: z.string().trim().length(3),
    maximumAmountMinor: z.string().regex(/^\d+$/),
    counterparty: z.string(),
  }),
]);
const jsonObjectSchema = z.record(z.string(), z.json());

type GrantForm = z.input<typeof grantFormSchema>;

const INITIAL_TOOL_FORM: GrantForm = {
  domain: 'tool',
  principal: '',
  capability: 'tool:',
  requirementIds: '',
  operation: '',
  maxUses: '',
  expiresAt: '',
  sideEffect: 'api_mutation',
  serverId: '',
  toolName: '',
  schemaHash: '',
  parameters: '{}',
};

export function AuthorizationGrantsContent({ workspaceSlug, environmentId, grants }: Props) {
  const router = useRouter();
  const [rows, setRows] = useState(grants);
  const [busyIds, setBusyIds] = useState<string[]>([]);
  const [creating, setCreating] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [form, setForm] = useState<GrantForm>(INITIAL_TOOL_FORM);
  const busy = useMemo(() => new Set(busyIds), [busyIds]);
  const query = `?workspace=${encodeURIComponent(workspaceSlug)}&environment=${encodeURIComponent(environmentId)}`;

  async function revoke(id: string) {
    setBusyIds((current) => [...current, id]);
    try {
      const response = await fetch(
        `/api/authorization/grants/${encodeURIComponent(id)}/revoke${query}`,
        { method: 'POST' },
      );
      const body = await response.text();
      if (!response.ok) throw new Error(`Unable to revoke grant (${response.status})`);
      const revoked = revokedGrantSchema.parse(JSON.parse(body));
      setRows((current) =>
        current.map((grant) =>
          grant.id === revoked.id ? { ...grant, status: revoked.status } : grant,
        ),
      );
      toast.success('Grant revoked');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Unable to revoke grant');
    } finally {
      setBusyIds((current) => current.filter((candidate) => candidate !== id));
    }
  }

  async function createGrant() {
    setCreating(true);
    try {
      const input = grantFormSchema.parse(form);
      const requirementIds = input.requirementIds
        .split(',')
        .map((value) => value.trim())
        .filter(Boolean);
      if (requirementIds.length === 0) throw new Error('At least one requirement ID is required');
      const common = {
        principal_id: input.principal,
        domain: input.domain,
        capability: input.capability,
        requirement_ids: requirementIds,
        ...(input.maxUses ? { max_uses: Number(input.maxUses) } : {}),
        ...(input.expiresAt ? { expires_at: new Date(input.expiresAt).toISOString() } : {}),
      };
      const scope =
        input.domain === 'tool'
          ? {
              scope_type: 'action',
              scope: {
                operations: [input.operation],
                side_effects: [input.sideEffect],
                ...(input.serverId ? { server_id: input.serverId } : {}),
                ...(input.toolName ? { tool_name: input.toolName } : {}),
                ...(input.schemaHash ? { schema_hash: input.schemaHash } : {}),
                parameters: jsonObjectSchema.parse(JSON.parse(input.parameters)),
                allowed_destinations: [],
              },
            }
          : {
              scope_type: 'financial',
              scope: {
                action_kinds: [input.actionKind],
                operation: input.operation,
                rail: input.rail,
                currency: input.currency.toUpperCase(),
                maximum_amount_minor: Number(input.maximumAmountMinor),
                counterparties: input.counterparty ? [input.counterparty] : [],
                x402_hosts: [],
                x402_resources: [],
                x402_networks: [],
                x402_assets: [],
                x402_payees: [],
                required_preconditions: [],
              },
            };
      const response = await fetch(`/api/authorization/grants${query}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ...common, scope }),
      });
      if (!response.ok) throw new Error(`Unable to create grant (${response.status})`);
      setShowCreate(false);
      setForm(INITIAL_TOOL_FORM);
      toast.success('Grant created', {
        description: 'Current policy and live domain checks still apply to every use.',
      });
      router.refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Unable to create grant');
    } finally {
      setCreating(false);
    }
  }

  const columns: DataTableColumn<AuthorizationGrant>[] = [
    {
      id: 'capability',
      header: 'Capability',
      cell: (row) => (
        <div className="grid gap-1">
          <span className="font-medium">{row.capability}</span>
          <span className="font-mono text-xs text-muted-foreground">{row.principal_id}</span>
        </div>
      ),
    },
    {
      id: 'domain',
      header: 'Domain',
      cell: (row) => <Badge variant="outline">{row.domain}</Badge>,
    },
    { id: 'source', header: 'Source', cell: (row) => row.source.replaceAll('_', ' ') },
    {
      id: 'scope',
      header: 'Policy-bounded scope',
      cell: (row) => (
        <pre className="max-w-md overflow-auto rounded-lg bg-muted p-3 text-xs">
          {JSON.stringify(row.scope ?? { exact_fingerprint: row.exact_fingerprint }, null, 2)}
        </pre>
      ),
    },
    {
      id: 'usage',
      header: 'Usage',
      cell: (row) => `${row.use_count}${row.max_uses == null ? '' : ` / ${row.max_uses}`}`,
    },
    {
      id: 'status',
      header: 'Status',
      cell: (row) => <Badge variant="outline">{row.status}</Badge>,
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cell: (row) =>
        row.status === 'active' ? (
          <Button
            size="sm"
            variant="outline"
            disabled={busy.has(row.id)}
            onClick={() => revoke(row.id)}
          >
            <IconBan /> Revoke
          </Button>
        ) : null,
    },
  ];

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Unified authorization"
        title="Grants"
        description="Reusable user or reviewer authority. Every use is intersected with the request and current policy boundary."
        actions={
          <Button onClick={() => setShowCreate(true)}>
            <IconPlus /> Create grant
          </Button>
        }
      />
      <Card>
        <CardHeader>
          <CardTitle>Active and historical grants</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={columns}
            rows={rows}
            getRowKey={(row) => row.id}
            caption="Authorization grants"
            empty={
              <EmptyState
                title="No grants"
                description="Approved reusable authority will appear here."
              />
            }
          />
        </CardContent>
      </Card>
      <Dialog open={showCreate} onOpenChange={(open) => !creating && setShowCreate(open)}>
        <DialogContent className="max-h-[90vh] max-w-2xl overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Create policy-bounded authority</DialogTitle>
            <DialogDescription>
              This grant can satisfy only the named requirements and typed scope. It cannot override
              denial, deferral, or current policy.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Domain">
              <select
                className="h-9 w-full rounded-md border bg-background px-3 text-sm"
                value={form.domain}
                onChange={(event) => {
                  if (event.target.value === 'financial') {
                    setForm({
                      domain: 'financial',
                      principal: '',
                      capability: 'financial:',
                      requirementIds: '',
                      operation: '',
                      maxUses: '',
                      expiresAt: '',
                      actionKind: 'refund',
                      rail: 'payment_http',
                      currency: 'USD',
                      maximumAmountMinor: '',
                      counterparty: '',
                    });
                  } else setForm(INITIAL_TOOL_FORM);
                }}
              >
                <option value="tool">Tool / action</option>
                <option value="financial">Financial</option>
              </select>
            </Field>
            <Field label="Principal">
              <Input
                value={form.principal}
                onChange={(e) => setForm({ ...form, principal: e.target.value })}
              />
            </Field>
            <Field label="Capability">
              <Input
                value={form.capability}
                onChange={(e) => setForm({ ...form, capability: e.target.value })}
              />
            </Field>
            <Field label="Requirement IDs (comma-separated)">
              <Input
                value={form.requirementIds}
                onChange={(e) => setForm({ ...form, requirementIds: e.target.value })}
              />
            </Field>
            <Field label="Operation">
              <Input
                value={form.operation}
                onChange={(e) => setForm({ ...form, operation: e.target.value })}
              />
            </Field>
            <Field label="Maximum uses (optional)">
              <Input
                inputMode="numeric"
                value={form.maxUses}
                onChange={(e) => setForm({ ...form, maxUses: e.target.value })}
              />
            </Field>
            <Field label="Expires at (optional)">
              <Input
                type="datetime-local"
                value={form.expiresAt}
                onChange={(e) => setForm({ ...form, expiresAt: e.target.value })}
              />
            </Field>
            {form.domain === 'tool' ? (
              <>
                <Field label="Side effect">
                  <select
                    className="h-9 w-full rounded-md border bg-background px-3 text-sm"
                    value={form.sideEffect}
                    onChange={(e) =>
                      setForm({
                        ...form,
                        sideEffect: grantFormSchema.options[0].shape.sideEffect.parse(
                          e.target.value,
                        ),
                      })
                    }
                  >
                    {[
                      'none',
                      'read',
                      'external_communication',
                      'file_write',
                      'shell_exec',
                      'network_call',
                      'db_mutation',
                      'api_mutation',
                      'memory_write',
                      'publish',
                    ].map((value) => (
                      <option key={value} value={value}>
                        {value.replaceAll('_', ' ')}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Server ID">
                  <Input
                    value={form.serverId}
                    onChange={(e) => setForm({ ...form, serverId: e.target.value })}
                  />
                </Field>
                <Field label="Tool name">
                  <Input
                    value={form.toolName}
                    onChange={(e) => setForm({ ...form, toolName: e.target.value })}
                  />
                </Field>
                <Field label="Schema hash">
                  <Input
                    value={form.schemaHash}
                    onChange={(e) => setForm({ ...form, schemaHash: e.target.value })}
                  />
                </Field>
                <Field label="Exact canonical parameters" wide>
                  <Input
                    value={form.parameters}
                    onChange={(e) => setForm({ ...form, parameters: e.target.value })}
                  />
                </Field>
              </>
            ) : (
              <>
                <Field label="Action kind">
                  <Input
                    value={form.actionKind}
                    onChange={(e) =>
                      setForm({
                        ...form,
                        actionKind: grantFormSchema.options[1].shape.actionKind.parse(
                          e.target.value,
                        ),
                      })
                    }
                  />
                </Field>
                <Field label="Rail">
                  <Input
                    value={form.rail}
                    onChange={(e) =>
                      setForm({
                        ...form,
                        rail: grantFormSchema.options[1].shape.rail.parse(e.target.value),
                      })
                    }
                  />
                </Field>
                <Field label="Currency">
                  <Input
                    value={form.currency}
                    onChange={(e) => setForm({ ...form, currency: e.target.value })}
                  />
                </Field>
                <Field label="Maximum amount (minor units)">
                  <Input
                    inputMode="numeric"
                    value={form.maximumAmountMinor}
                    onChange={(e) => setForm({ ...form, maximumAmountMinor: e.target.value })}
                  />
                </Field>
                <Field label="Counterparty ID">
                  <Input
                    value={form.counterparty}
                    onChange={(e) => setForm({ ...form, counterparty: e.target.value })}
                  />
                </Field>
              </>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" disabled={creating} onClick={() => setShowCreate(false)}>
              Cancel
            </Button>
            <Button disabled={creating} onClick={createGrant}>
              Create grant
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Field({
  label,
  wide = false,
  children,
}: {
  label: string;
  wide?: boolean;
  children: ReactNode;
}) {
  return (
    <Label className={wide ? 'grid gap-2 sm:col-span-2' : 'grid gap-2'}>
      <span>{label}</span>
      {children}
    </Label>
  );
}
