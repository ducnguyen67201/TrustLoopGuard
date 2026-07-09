'use client';

import { IconBan, IconKey, IconPlus } from '@tabler/icons-react';
import { useState, type Dispatch, type FormEvent, type SetStateAction } from 'react';
import { toast } from 'sonner';
import { z } from 'zod';
import type { FinancialMandate } from '@trustloopguard/sdk';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import { FinancialAuthorizationModel } from '@/components/workspace/FinancialAuthorizationModel';
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

type ManagedMandateForm = {
  principalId: string;
  rawRequest: string;
  intentLabel: string;
  maxAmount: string;
  currency: string;
  host: string;
  resource: string;
  network: string;
  asset: string;
  payTo: string;
};

const DEFAULT_MANAGED_FORM: ManagedMandateForm = {
  principalId: 'spid:commerce-agent',
  rawRequest: 'User asked: "Buy access to the premium article about agentic commerce."',
  intentLabel:
    'Allow spid:commerce-agent to pay up to $5.00 for /premium/article/agentic-commerce on the sandbox merchant.',
  maxAmount: '5.00',
  currency: 'USD',
  host: '127.0.0.1:4021',
  resource: '/premium/article/agentic-commerce',
  network: 'base-sepolia',
  asset: 'USDC',
  payTo: '0xabc1230000000000000000000000000000000000',
};

const DEFAULT_ADVANCED_SCOPE = JSON.stringify(
  {
    action_kinds: ['payment'],
    currency: 'USD',
    max_amount_minor: 500,
    allowed_hosts: ['127.0.0.1:4021'],
    allowed_resources: ['/premium/article/agentic-commerce'],
    allowed_networks: ['base-sepolia'],
    allowed_assets: ['USDC'],
    allowed_pay_to: ['0xabc1230000000000000000000000000000000000'],
  },
  null,
  2,
);

export function FinancialMandatesContent({
  workspaceSlug,
  environmentId,
  mandates,
}: FinancialMandatesContentProps) {
  const [rows, setRows] = useState(mandates);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [managedForm, setManagedForm] = useState(DEFAULT_MANAGED_FORM);
  const [advancedPrincipalId, setAdvancedPrincipalId] = useState('spid:commerce-agent');
  const [advancedScope, setAdvancedScope] = useState(DEFAULT_ADVANCED_SCOPE);
  const [submitting, setSubmitting] = useState(false);
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);

  const columns: DataTableColumn<FinancialMandate>[] = [
    {
      id: 'principal',
      header: 'Agent',
      cell: (row) => (
        <div className="grid min-w-36 gap-0.5">
          <span className="truncate font-mono text-xs text-foreground">{row.principal_id}</span>
          <span className="truncate font-mono text-xs text-muted-foreground">{row.id}</span>
        </div>
      ),
    },
    {
      id: 'intent',
      header: 'User request',
      cell: (row) => <MandateIntentSummary mandate={row} />,
    },
    {
      id: 'scope',
      header: 'Boundary',
      cell: (row) => <MandateScopeSummary mandate={row} />,
    },
    {
      id: 'status',
      header: 'Status',
      cell: (row) => (
        <div className="flex items-center gap-1.5">
          <MandateStatusBadge mandate={row} />
          <span className="font-mono text-2xs text-muted-foreground">v{row.version}</span>
        </div>
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

  async function createManagedMandate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const parsed = managedMandateSchema.parse(managedForm);
      const amountMinor = dollarsToMinor(parsed.maxAmount);
      const paymentScope = {
        intent_label: parsed.intentLabel,
        action_kinds: ['payment'],
        operation: 'x402_read_paid_resource',
        max_amount_minor: amountMinor,
        currency: parsed.currency.toUpperCase(),
        rail: 'x402',
        allowed_hosts: [parsed.host],
        allowed_resources: [parsed.resource],
        allowed_networks: [parsed.network],
        allowed_assets: [parsed.asset],
        allowed_pay_to: [parsed.payTo],
        allowed_counterparty_ids: [parsed.payTo],
        required_preconditions: [],
      };
      await createMandate({
        principal_id: parsed.principalId,
        scope: {},
        payment_scope: paymentScope,
        metadata: {
          source: 'dashboard',
          mandate_mode: 'trustloop_managed',
          raw_user_request: parsed.rawRequest,
          user_intent: parsed.intentLabel,
        },
      });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Mandate create failed');
    } finally {
      setSubmitting(false);
    }
  }

  async function createAdvancedMandate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const parsedScope = mandateScopeSchema.parse(JSON.parse(advancedScope));
      await createMandate({
        principal_id: advancedPrincipalId.trim(),
        scope: parsedScope,
        metadata: { source: 'dashboard', mandate_mode: 'advanced_json' },
      });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Mandate create failed');
    } finally {
      setSubmitting(false);
    }
  }

  async function createMandate(body: Record<string, unknown>) {
    const response = await fetch(`/api/financial/mandates${contextQuery}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    const text = await response.text();
    if (!response.ok) throw new Error(safeError(text) ?? 'Unable to create mandate');
    const mandate = JSON.parse(text) as FinancialMandate;
    setRows((prev) => [mandate, ...prev]);
    toast.success('Mandate created');
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
        title="User intent mandates"
        description="Translate a user request into a payment boundary the agent must present before signing."
      />
      <FinancialAuthorizationModel active="mandates" contextQuery={contextQuery} />
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(22rem,28rem)]">
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
                  description="Create the user intent boundary first, then require it from the x402 financial policy."
                />
              }
              caption="Financial mandates"
            />
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Create internal mandate</CardTitle>
          </CardHeader>
          <CardContent>
            <Tabs defaultValue="managed" className="gap-4">
              <TabsList className="w-full">
                <TabsTrigger value="managed">
                  <IconKey />
                  Guided
                </TabsTrigger>
                <TabsTrigger value="advanced">Scope JSON</TabsTrigger>
              </TabsList>
              <TabsContent value="managed">
                <p className="mb-3 text-sm text-muted-foreground">
                  Store the raw request and the interpreted payment boundary together.
                </p>
                <form onSubmit={createManagedMandate} className="grid gap-3">
                  <ManagedField
                    label="Agent principal"
                    value={managedForm.principalId}
                    onChange={(value) => setManagedFormValue(setManagedForm, 'principalId', value)}
                    mono
                  />
                  <ManagedTextarea
                    label="Raw user request"
                    value={managedForm.rawRequest}
                    onChange={(value) => setManagedFormValue(setManagedForm, 'rawRequest', value)}
                  />
                  <ManagedField
                    label="Interpreted mandate"
                    value={managedForm.intentLabel}
                    onChange={(value) => setManagedFormValue(setManagedForm, 'intentLabel', value)}
                  />
                  <div className="grid gap-3 sm:grid-cols-2">
                    <ManagedField
                      label="Max amount"
                      value={managedForm.maxAmount}
                      onChange={(value) => setManagedFormValue(setManagedForm, 'maxAmount', value)}
                      mono
                    />
                    <ManagedField
                      label="Currency"
                      value={managedForm.currency}
                      onChange={(value) => setManagedFormValue(setManagedForm, 'currency', value)}
                      mono
                    />
                  </div>
                  <ManagedField
                    label="Merchant host"
                    value={managedForm.host}
                    onChange={(value) => setManagedFormValue(setManagedForm, 'host', value)}
                    mono
                  />
                  <ManagedField
                    label="Resource"
                    value={managedForm.resource}
                    onChange={(value) => setManagedFormValue(setManagedForm, 'resource', value)}
                    mono
                  />
                  <div className="grid gap-3 sm:grid-cols-2">
                    <ManagedField
                      label="Network"
                      value={managedForm.network}
                      onChange={(value) => setManagedFormValue(setManagedForm, 'network', value)}
                      mono
                    />
                    <ManagedField
                      label="Asset"
                      value={managedForm.asset}
                      onChange={(value) => setManagedFormValue(setManagedForm, 'asset', value)}
                      mono
                    />
                  </div>
                  <ManagedField
                    label="Pay to"
                    value={managedForm.payTo}
                    onChange={(value) => setManagedFormValue(setManagedForm, 'payTo', value)}
                    mono
                  />
                  <Button type="submit" disabled={submitting}>
                    <IconPlus />
                    Create managed mandate
                  </Button>
                </form>
              </TabsContent>
              <TabsContent value="advanced">
                <p className="mb-3 text-sm text-muted-foreground">
                  Use the same normalized scope shape when another system created the user intent
                  and TrustLoopGuard stores the boundary for authorization.
                </p>
                <form onSubmit={createAdvancedMandate} className="grid gap-3">
                  <ManagedField
                    label="Agent principal"
                    value={advancedPrincipalId}
                    onChange={setAdvancedPrincipalId}
                    mono
                  />
                  <div className="grid gap-2">
                    <Label htmlFor="mandate-scope">Scope JSON</Label>
                    <Textarea
                      id="mandate-scope"
                      required
                      value={advancedScope}
                      onChange={(event) => setAdvancedScope(event.target.value)}
                      className="min-h-48 font-mono text-xs"
                    />
                  </div>
                  <Button type="submit" disabled={submitting}>
                    <IconPlus />
                    Create JSON mandate
                  </Button>
                </form>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function MandateScopeSummary({ mandate }: { mandate: FinancialMandate }) {
  const scope = mandate.scope as Record<string, unknown> | null;
  const maxAmount =
    typeof scope?.['max_amount_minor'] === 'number' ? scope['max_amount_minor'] : null;
  const currency = typeof scope?.['currency'] === 'string' ? scope['currency'] : null;
  const resource = firstString(scope?.['allowed_resources']);
  const host = firstString(scope?.['allowed_hosts']);
  return (
    <div className="grid min-w-0 gap-1">
      <div className="flex flex-wrap gap-1">
        <Badge variant="outline">{currency ?? 'Currency'}</Badge>
        <Badge variant="outline">
          {maxAmount === null ? 'Bounded' : `$${(maxAmount / 100).toFixed(2)}`}
        </Badge>
      </div>
      <span className="truncate font-mono text-xs text-muted-foreground">
        {[host, resource].filter(Boolean).join(' ') || JSON.stringify(scope)}
      </span>
    </div>
  );
}

function MandateIntentSummary({ mandate }: { mandate: FinancialMandate }) {
  const metadata = mandate.metadata as Record<string, unknown> | null;
  const rawRequest = stringMetadata(metadata, 'raw_user_request');
  const interpreted = stringMetadata(metadata, 'user_intent');
  const source = mandateSource(metadata);
  return (
    <div className="grid min-w-56 max-w-md gap-1">
      <span className="line-clamp-2 text-sm text-foreground">
        {rawRequest ?? interpreted ?? 'No raw request recorded'}
      </span>
      <div className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
        <span
          className={
            source.kind === 'internal'
              ? 'rounded-sm bg-[var(--badge-allow-bg)] px-1.5 py-0.5 text-[var(--color-allow)]'
              : 'rounded-sm border px-1.5 py-0.5 text-foreground'
          }
        >
          {source.label}
        </span>
        <span className="truncate">
          {interpreted ? `Interpreted: ${interpreted}` : 'Interpreted boundary stored in scope'}
        </span>
      </div>
    </div>
  );
}

function mandateSource(metadata: Record<string, unknown> | null): {
  kind: 'internal' | 'external';
  label: string;
  detail: string;
} {
  const mode = stringMetadata(metadata, 'mandate_mode');
  const source = stringMetadata(metadata, 'source');
  if (mode === 'external_signed' || source === 'external') {
    return { kind: 'external', label: 'External', detail: 'supplied by customer app' };
  }
  return { kind: 'internal', label: 'Internal', detail: 'stored by TrustLoopGuard' };
}

function ManagedField({
  label,
  value,
  onChange,
  mono = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  mono?: boolean;
}) {
  const id = `mandate-${label.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        required
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={mono ? 'font-mono' : undefined}
      />
    </div>
  );
}

function ManagedTextarea({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  const id = `mandate-${label.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Textarea
        id={id}
        required
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="min-h-20"
      />
    </div>
  );
}

function setManagedFormValue(
  setForm: Dispatch<SetStateAction<ManagedMandateForm>>,
  key: keyof ManagedMandateForm,
  value: string,
) {
  setForm((prev) => ({ ...prev, [key]: value }));
}

function dollarsToMinor(value: string): number {
  const amount = Number(value);
  if (!Number.isFinite(amount) || amount <= 0) {
    throw new Error('Max amount must be greater than zero');
  }
  return Math.round(amount * 100);
}

function firstString(value: unknown): string | null {
  if (!Array.isArray(value)) return null;
  const first = value.find((item): item is string => typeof item === 'string');
  return first ?? null;
}

function stringMetadata(metadata: Record<string, unknown> | null | undefined, key: string) {
  const value = metadata?.[key];
  return typeof value === 'string' && value.trim() !== '' ? value : null;
}

const managedMandateSchema = z.object({
  principalId: z.string().trim().min(1, 'Agent principal is required'),
  rawRequest: z.string().trim().min(1, 'Raw user request is required'),
  intentLabel: z.string().trim().min(1, 'User intent is required'),
  maxAmount: z.string().trim().min(1, 'Max amount is required'),
  currency: z.string().trim().min(1, 'Currency is required'),
  host: z.string().trim().min(1, 'Merchant host is required'),
  resource: z.string().trim().min(1, 'Resource is required'),
  network: z.string().trim().min(1, 'Network is required'),
  asset: z.string().trim().min(1, 'Asset is required'),
  payTo: z.string().trim().min(1, 'Pay-to address is required'),
});

const mandateScopeSchema = z.looseObject({
  action_kinds: z.array(z.string()).min(1, 'Scope must include at least one action kind'),
  currency: z.string().trim().min(1, 'Scope currency is required'),
  max_amount_minor: z.number().int().positive().optional(),
});
