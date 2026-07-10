'use client';

import {
  IconAlertTriangle,
  IconCircleCheck,
  IconCopy,
  IconEdit,
  IconPlugConnected,
  IconPlus,
  IconRoute,
  IconTrash,
} from '@tabler/icons-react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useRef, useState, type FormEvent, type ReactNode } from 'react';
import { toast } from 'sonner';
import type {
  GatewayProviderConnection,
  GatewayProviderKind,
  GatewayRoute,
} from '@trustloopguard/sdk';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { EmptyState } from '@/components/ui/empty-state';
import { InfoHint } from '@/components/ui/info-hint';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { GatewayPageData } from '@/lib/server/dashboard-data';

type ProviderConnection = GatewayProviderConnection;

type RouteReadiness = {
  label: string;
  tone: 'ready' | 'warning';
};

export function GatewayPageContent({
  data,
  apiBaseUrl,
  budgetReadiness = { hasPrice: false, hasCap: false, hasAlert: false },
}: {
  data: GatewayPageData;
  apiBaseUrl: string;
  budgetReadiness?: { hasPrice: boolean; hasCap: boolean; hasAlert: boolean };
}) {
  const router = useRouter();
  const [selectedRouteId, setSelectedRouteId] = useState(data.gatewayRoutes[0]?.id ?? '');
  const [deleteTarget, setDeleteTarget] = useState<ProviderConnection | null>(null);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    setSelectedRouteId((current) => {
      if (data.gatewayRoutes.some((route) => route.id === current)) return current;
      return data.gatewayRoutes[0]?.id ?? '';
    });
  }, [data.gatewayRoutes]);

  const selectedRoute = data.gatewayRoutes.find((route) => route.id === selectedRouteId) ?? null;
  const routeIdForSnippet = selectedRoute?.id ?? '<route_id>';
  const normalizedApiBaseUrl = apiBaseUrl.replace(/\/$/, '');
  const openAiBaseUrl = `${normalizedApiBaseUrl}/v1/gateway/${routeIdForSnippet}/openai`;
  const anthropicBaseUrl = `${normalizedApiBaseUrl}/v1/gateway/${routeIdForSnippet}/anthropic`;

  const providerColumns: DataTableColumn<ProviderConnection>[] = [
    {
      id: 'name',
      header: 'Name',
      cell: (row) => <span className="font-medium text-foreground">{row.display_name}</span>,
    },
    {
      id: 'kind',
      header: 'AI service',
      cell: (row) => providerKindLabel(row.kind),
    },
    {
      id: 'model',
      header: (
        <HeaderHint label="Default model">
          The AI model used when a request doesn&apos;t name one — for example, gpt-4o-mini.
        </HeaderHint>
      ),
      cell: (row) => <span className="font-mono text-xs">{row.default_model}</span>,
    },
    {
      id: 'credential',
      header: (
        <HeaderHint label="Key status">
          Whether the secret key you entered is stored and ready to use.
        </HeaderHint>
      ),
      align: 'right',
      cell: (row) => <CredentialBadge status={row.credential_status} />,
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cell: (row) => (
        <div className="flex justify-end gap-1">
          <ProviderConnectionDialog
            workspaceSlug={data.activeWorkspace.slug}
            provider={row}
            trigger={
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label={`Edit ${row.display_name}`}
              >
                <IconEdit />
              </Button>
            }
          />
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={`Delete ${row.display_name}`}
            onClick={() => setDeleteTarget(row)}
            className="text-destructive hover:text-destructive"
          >
            <IconTrash />
          </Button>
        </div>
      ),
    },
  ];

  async function confirmDeleteProvider() {
    if (deleteTarget === null || deleting) return;
    const provider = deleteTarget;
    setDeleting(true);
    try {
      await deleteGatewayConfig(
        `/api/gateway/provider-connections/${encodeURIComponent(provider.id)}${query(data.activeWorkspace.slug)}`,
      );
      toast.success('Provider permanently deleted');
      setDeleteTarget(null);
      router.refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Request failed');
    } finally {
      setDeleting(false);
    }
  }

  const routeColumns: DataTableColumn<GatewayRoute>[] = [
    {
      id: 'route',
      header: 'Route',
      cell: (row) => (
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate font-medium text-foreground">{row.display_name}</span>
          <span className="flex min-w-0 items-center gap-1 text-xs text-muted-foreground">
            <span className="shrink-0 font-sans text-muted-foreground/80">Address</span>
            <span className="truncate font-mono">{row.id}</span>
            <CopyButton
              value={row.id}
              label={`Copy address for ${row.display_name}`}
              successMessage="Address copied"
            />
          </span>
        </div>
      ),
    },
    {
      id: 'provider',
      header: (
        <HeaderHint label="AI service">
          The AI service this route forwards requests to, like OpenAI or Anthropic.
        </HeaderHint>
      ),
      cell: (row) => (
        <span className="text-sm">
          {nameFor(data.providerConnections, row.provider_connection_id)}
        </span>
      ),
    },
    {
      id: 'agent',
      header: (
        <HeaderHint label="Agent">
          The AI assistant or app whose traffic flows through this route.
        </HeaderHint>
      ),
      cell: (row) => <span className="text-sm">{nameForAgent(data.agents, row.agent_id)}</span>,
    },
    {
      id: 'status',
      header: 'Status',
      align: 'right',
      cell: (row) => {
        const readiness = routeReadiness(data, row);
        return (
          <Badge variant={readiness.tone === 'ready' ? 'allow' : 'escalate'}>
            {readiness.tone === 'ready' ? <IconCircleCheck /> : <IconAlertTriangle />}
            {readiness.label}
          </Badge>
        );
      },
    },
  ];

  const providerCount = data.providerConnections.length;
  const routeCount = data.gatewayRoutes.length;
  const hasRuntimeKey = data.activeRuntimeKeyCount > 0;
  const liveRoutes = data.gatewayRoutes.filter(
    (route) => routeReadiness(data, route).tone === 'ready',
  ).length;

  // Open on the first section that already has something to act on, so the
  // operator never lands on an empty tab when other tabs are populated.
  const defaultTab = routeCount > 0 ? 'routes' : providerCount > 0 ? 'providers' : 'routes';

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow={data.activeWorkspace.name}
        title="Gateway"
        help={<InfoHint term="gateway" />}
        description="Route AI provider calls through TrustLoopGuard so every request uses the same enabled policies as your SDK and event traffic."
        actions={
          <GatewayRouteDialog
            workspaceSlug={data.activeWorkspace.slug}
            providers={data.providerConnections}
            agents={data.agents}
          />
        }
      />

      <section className="grid gap-3 sm:grid-cols-3">
        <StatCard
          icon={<IconRoute />}
          label="Routes"
          value={routeCount}
          hint={routeCount > 0 ? `${liveRoutes} ready to use` : 'None set up yet'}
        />
        <StatCard
          icon={<IconPlugConnected />}
          label="Providers"
          value={providerCount}
          hint={providerCount > 0 ? 'AI services connected' : 'None connected yet'}
        />
        <StatCard
          icon={hasRuntimeKey ? <IconCircleCheck /> : <IconAlertTriangle />}
          label="API keys"
          value={data.activeRuntimeKeyCount}
          tone={hasRuntimeKey ? 'default' : 'warning'}
          hint={hasRuntimeKey ? 'Your app can connect' : 'Needed before traffic flows'}
        />
      </section>

      {!hasRuntimeKey ? (
        <Alert>
          <IconAlertTriangle />
          <AlertTitle>You need an API key first</AlertTitle>
          <AlertDescription>
            Your app uses an API key to connect to the gateway. Without one, traffic can&apos;t flow
            through yet.
            <Button asChild variant="link" className="h-auto px-1 py-0">
              <Link href={`/api-keys?workspace=${encodeURIComponent(data.activeWorkspace.slug)}`}>
                Create an API key to get started.
              </Link>
            </Button>
          </AlertDescription>
        </Alert>
      ) : null}

      <Alert>
        {budgetReadiness.hasPrice && budgetReadiness.hasCap ? (
          <IconCircleCheck />
        ) : (
          <IconAlertTriangle />
        )}
        <AlertTitle>Provider spending controls</AlertTitle>
        <AlertDescription className="flex flex-wrap items-center gap-x-3 gap-y-1">
          <span>{budgetReadiness.hasPrice ? 'Model price ready' : 'Model price needed'}</span>
          <span>{budgetReadiness.hasCap ? 'Hard cap ready' : 'Hard cap not configured'}</span>
          <span>{budgetReadiness.hasAlert ? 'Alert ready' : '80% alert not configured'}</span>
          <Button asChild variant="link" className="h-auto px-0 py-0">
            <Link
              href={`/usage?workspace=${encodeURIComponent(data.activeWorkspace.slug)}&environment=${encodeURIComponent(data.activeEnvironment.id)}#budgets`}
            >
              Configure Usage &amp; budgets
            </Link>
          </Button>
        </AlertDescription>
      </Alert>

      <Tabs defaultValue={defaultTab} className="gap-4">
        <TabsList>
          <TabsTrigger value="routes">
            Routes
            <CountChip value={routeCount} />
          </TabsTrigger>
          <TabsTrigger value="providers">
            Providers
            <CountChip value={providerCount} />
          </TabsTrigger>
        </TabsList>

        <TabsContent value="routes" className="grid gap-4">
          <SectionCard
            title="Routes"
            help={<InfoHint term="route" />}
            description="A route connects one provider and one agent. All enabled policies for this environment and agent apply automatically."
            action={
              routeCount > 0 ? (
                <GatewayRouteDialog
                  workspaceSlug={data.activeWorkspace.slug}
                  providers={data.providerConnections}
                  agents={data.agents}
                />
              ) : null
            }
          >
            {routeCount > 0 ? (
              <DataTable
                columns={routeColumns}
                rows={data.gatewayRoutes}
                getRowKey={(route) => route.id}
                caption="Gateway routes in this workspace"
                empty="No routes yet."
              />
            ) : (
              <EmptyState
                icon={<IconRoute />}
                title="No routes set up yet"
                description="Connect a provider and choose the agent whose traffic this route represents. Enabled policies apply automatically."
                action={
                  <GatewayRouteDialog
                    workspaceSlug={data.activeWorkspace.slug}
                    providers={data.providerConnections}
                    agents={data.agents}
                  />
                }
              />
            )}
          </SectionCard>
          {routeCount > 0 ? (
            <IntegrationCard
              routes={data.gatewayRoutes}
              selectedRouteId={selectedRouteId}
              onSelectedRouteIdChange={setSelectedRouteId}
              openAiBaseUrl={openAiBaseUrl}
              anthropicBaseUrl={anthropicBaseUrl}
              hasRuntimeKey={hasRuntimeKey}
              workspaceSlug={data.activeWorkspace.slug}
              providers={data.providerConnections}
              agents={data.agents}
            />
          ) : null}
        </TabsContent>

        <TabsContent value="providers">
          <SectionCard
            title="Providers"
            help={<InfoHint term="provider" />}
            description="The AI services you connect, like OpenAI or Anthropic. The gateway forwards your traffic to them. Your secret keys are stored securely and never shown again."
            action={
              providerCount > 0 ? (
                <ProviderConnectionDialog workspaceSlug={data.activeWorkspace.slug} />
              ) : null
            }
          >
            {providerCount > 0 ? (
              <DataTable
                columns={providerColumns}
                rows={data.providerConnections}
                getRowKey={(provider) => provider.id}
                caption="Provider connections in this workspace"
                empty="No providers yet."
              />
            ) : (
              <EmptyState
                icon={<IconPlugConnected />}
                title="No providers connected yet"
                description="Connect an AI service such as OpenAI or Anthropic so the gateway has somewhere to send your requests. This is the first step to start routing traffic."
                action={<ProviderConnectionDialog workspaceSlug={data.activeWorkspace.slug} />}
              />
            )}
          </SectionCard>
        </TabsContent>
      </Tabs>

      <AlertDialog
        open={deleteTarget !== null}
        onOpenChange={(open) => !open && !deleting && setDeleteTarget(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Permanently delete this provider?</AlertDialogTitle>
            <AlertDialogDescription>
              {deleteTarget?.display_name ?? 'This provider'} and its stored secret key will be
              permanently deleted. This cannot be undone. Providers used by a route must be removed
              from that route first.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleting}>Keep provider</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              disabled={deleting}
              onClick={confirmDeleteProvider}
            >
              {deleting ? 'Deleting...' : 'Delete provider'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

function ProviderConnectionDialog({
  workspaceSlug,
  provider,
  trigger,
}: {
  workspaceSlug: string;
  provider?: ProviderConnection;
  trigger?: ReactNode;
}) {
  const router = useRouter();
  const editing = provider !== undefined;
  const [open, setOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [id, setId] = useState('');
  const [displayName, setDisplayName] = useState(provider?.display_name ?? '');
  const [kind, setKind] = useState<GatewayProviderKind>(provider?.kind ?? 'openai_compatible');
  const [baseUrl, setBaseUrl] = useState(provider?.base_url ?? '');
  const [defaultModel, setDefaultModel] = useState(provider?.default_model ?? 'gpt-4o-mini');
  const [providerApiKey, setProviderApiKey] = useState('');
  const isPaymentProvider = kind === 'payment_http';

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    try {
      const body = {
        ...(!editing && id.trim() !== '' ? { id: id.trim() } : {}),
        display_name: displayName.trim(),
        ...(!editing ? { kind } : {}),
        ...(editing || baseUrl.trim() !== '' ? { base_url: baseUrl.trim() } : {}),
        ...(!isPaymentProvider ? { default_model: defaultModel.trim() } : {}),
        ...(providerApiKey.trim() !== '' ? { provider_api_key: providerApiKey.trim() } : {}),
      };
      const url = editing
        ? `/api/gateway/provider-connections/${encodeURIComponent(provider.id)}${query(workspaceSlug)}`
        : `/api/gateway/provider-connections${query(workspaceSlug)}`;
      await sendGatewayConfig(url, editing ? 'PATCH' : 'POST', body);
      toast.success(editing ? 'Provider updated' : 'Provider connection created');
      setOpen(false);
      reset();
      router.refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Request failed');
    } finally {
      setSubmitting(false);
    }
  }

  function reset() {
    setId('');
    setDisplayName(provider?.display_name ?? '');
    setKind(provider?.kind ?? 'openai_compatible');
    setBaseUrl(provider?.base_url ?? '');
    setDefaultModel(provider?.default_model ?? 'gpt-4o-mini');
    setProviderApiKey('');
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        {trigger ?? (
          <Button variant="outline">
            <IconPlus />
            Provider
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="sm:max-w-lg">
        <form onSubmit={onSubmit} className="grid gap-4">
          <DialogHeader>
            <DialogTitle>{editing ? 'Edit provider' : 'Connect a provider'}</DialogTitle>
            <DialogDescription>
              {editing
                ? 'Update this connection or enter a new secret key to rotate the stored credential.'
                : 'Connect an AI service like OpenAI or Anthropic so the gateway has somewhere to send your requests. Your secret key is stored securely and never shown again.'}
            </DialogDescription>
          </DialogHeader>
          {!editing ? (
            <Field
              label="Short id"
              htmlFor="provider-id"
              optional
              hint="A short, lowercase nickname used in links (for example, openai-prod). Leave blank and we'll create one for you."
            >
              <Input
                id="provider-id"
                autoComplete="off"
                className="font-mono"
                placeholder="openai-prod"
                value={id}
                onChange={(event) => setId(event.target.value)}
              />
            </Field>
          ) : null}
          <Field
            label="Name"
            htmlFor="provider-name"
            hint="A friendly name you'll recognize in lists."
          >
            <Input
              id="provider-name"
              required
              autoComplete="off"
              placeholder="OpenAI production"
              value={displayName}
              onChange={(event) => setDisplayName(event.target.value)}
            />
          </Field>
          <Field
            label="AI service"
            htmlFor="provider-kind"
            hint="Pick the company that runs the model you want to use."
          >
            <Select
              value={kind}
              disabled={editing}
              onValueChange={(value) => {
                const nextKind = value as GatewayProviderKind;
                setKind(nextKind);
                setDefaultModel(
                  nextKind === 'anthropic' ? 'claude-3-5-sonnet-latest' : 'gpt-4o-mini',
                );
              }}
            >
              <SelectTrigger id="provider-kind" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="openai_compatible">OpenAI-compatible</SelectItem>
                <SelectItem value="anthropic">Anthropic</SelectItem>
                {isPaymentProvider ? (
                  <SelectItem value="payment_http">Payment HTTP</SelectItem>
                ) : null}
              </SelectContent>
            </Select>
          </Field>
          <Field
            label="Service address"
            htmlFor="provider-base-url"
            optional={!isPaymentProvider}
            hint="Only change this if you use a custom or self-hosted endpoint. Most people can leave it blank."
          >
            <Input
              id="provider-base-url"
              type="url"
              className="font-mono"
              placeholder={
                kind === 'anthropic' ? 'https://api.anthropic.com' : 'https://api.openai.com'
              }
              value={baseUrl}
              onChange={(event) => setBaseUrl(event.target.value)}
            />
          </Field>
          {!isPaymentProvider ? (
            <Field
              label="Default model"
              htmlFor="provider-model"
              hint="The model used when a request doesn't name one — for example, gpt-4o-mini."
            >
              <Input
                id="provider-model"
                required
                autoComplete="off"
                className="font-mono"
                value={defaultModel}
                onChange={(event) => setDefaultModel(event.target.value)}
              />
            </Field>
          ) : null}
          <Field
            label={editing ? 'New secret key' : 'Secret key'}
            htmlFor="provider-key"
            optional={editing}
            hint="The API key from your AI service account. We store it securely and never show it again."
          >
            <Input
              id="provider-key"
              required={!editing}
              type="password"
              autoComplete="off"
              className="font-mono"
              value={providerApiKey}
              onChange={(event) => setProviderApiKey(event.target.value)}
            />
          </Field>
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={
                submitting ||
                !displayName.trim() ||
                (!isPaymentProvider && !defaultModel.trim()) ||
                (isPaymentProvider && !baseUrl.trim()) ||
                (!editing && !providerApiKey.trim())
              }
            >
              {submitting
                ? editing
                  ? 'Saving...'
                  : 'Connecting...'
                : editing
                  ? 'Save changes'
                  : 'Connect provider'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function GatewayRouteDialog({
  workspaceSlug,
  providers,
  agents,
}: {
  workspaceSlug: string;
  providers: ProviderConnection[];
  agents: Array<{ id: string; name: string }>;
}) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [id, setId] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [providerId, setProviderId] = useState('');
  const [agentId, setAgentId] = useState('');

  const hasDependencies = providers.length > 0 && agents.length > 0;

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting || !hasDependencies) return;
    setSubmitting(true);
    try {
      await postGatewayConfig(`/api/gateway/routes${query(workspaceSlug)}`, {
        ...(id.trim() === '' ? {} : { id: id.trim() }),
        display_name: displayName.trim(),
        provider_connection_id: providerId,
        agent_id: agentId,
      });
      toast.success('Gateway route created');
      setOpen(false);
      reset();
      router.refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Request failed');
    } finally {
      setSubmitting(false);
    }
  }

  function reset() {
    setId('');
    setDisplayName('');
    setProviderId('');
    setAgentId('');
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <IconPlus />
          New route
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-lg">
        <form onSubmit={onSubmit} className="grid gap-4">
          <DialogHeader>
            <DialogTitle>Create a route</DialogTitle>
            <DialogDescription>
              A route is the address your app points at. Pick one provider and one agent; all
              enabled policies apply automatically.
            </DialogDescription>
          </DialogHeader>
          {!hasDependencies ? (
            <Alert>
              <IconAlertTriangle />
              <AlertTitle>A few things are needed first</AlertTitle>
              <AlertDescription>
                Connect at least one provider and add an agent before you can build a route.
              </AlertDescription>
            </Alert>
          ) : null}
          <Field
            label="Short id"
            htmlFor="route-id"
            optional
            hint="A short, lowercase nickname used in the route's address. Leave blank to generate one."
          >
            <Input
              id="route-id"
              autoComplete="off"
              className="font-mono"
              placeholder="support-prod"
              value={id}
              onChange={(event) => setId(event.target.value)}
            />
          </Field>
          <Field
            label="Name"
            htmlFor="route-name"
            hint="A friendly name you'll recognize in lists."
          >
            <Input
              id="route-name"
              required
              autoComplete="off"
              placeholder="Support production"
              value={displayName}
              onChange={(event) => setDisplayName(event.target.value)}
            />
          </Field>
          <EntitySelect
            label="Provider"
            id="route-provider"
            value={providerId}
            values={providers.map((provider) => ({
              id: provider.id,
              label: provider.display_name,
            }))}
            hint="The AI service this route forwards requests to."
            onValueChange={setProviderId}
          />
          <EntitySelect
            label="Agent"
            id="route-agent"
            value={agentId}
            values={agents}
            hint="The AI assistant or app whose traffic flows through this route."
            onValueChange={setAgentId}
          />
          <Alert>
            <IconCircleCheck />
            <AlertTitle>Policies are already connected</AlertTitle>
            <AlertDescription>
              Every enabled policy for this environment and agent will check this route.
              <Button asChild variant="link" className="h-auto px-1 py-0">
                <Link href={`/policies?workspace=${encodeURIComponent(workspaceSlug)}`}>
                  Review policies.
                </Link>
              </Button>
            </AlertDescription>
          </Alert>
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={
                submitting || !hasDependencies || !displayName.trim() || !providerId || !agentId
              }
            >
              {submitting ? 'Creating...' : 'Create route'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function StatCard({
  icon,
  label,
  value,
  hint,
  tone = 'default',
}: {
  icon: ReactNode;
  label: string;
  value: number;
  hint?: string;
  tone?: 'default' | 'warning';
}) {
  return (
    <Card className="gap-0 py-0">
      <CardContent className="flex items-start gap-3 p-4">
        <div
          className={
            tone === 'warning'
              ? 'flex size-9 shrink-0 items-center justify-center rounded-md bg-[color-mix(in_oklab,var(--color-escalate),transparent_88%)] text-[var(--color-escalate)] [&_svg]:size-5'
              : 'flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:size-5'
          }
        >
          {icon}
        </div>
        <div className="grid min-w-0 gap-0.5">
          <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {label}
          </div>
          <div className="font-mono text-2xl font-semibold leading-none tabular-nums text-foreground">
            {value}
          </div>
          {hint ? <div className="truncate text-xs text-muted-foreground">{hint}</div> : null}
        </div>
      </CardContent>
    </Card>
  );
}

function CountChip({ value }: { value: number }) {
  return (
    <span className="ml-0.5 inline-flex min-w-4 items-center justify-center rounded-sm bg-muted px-1 font-mono text-[0.625rem] tabular-nums text-muted-foreground">
      {value}
    </span>
  );
}

function SectionCard({
  title,
  help,
  description,
  action,
  children,
}: {
  title: string;
  help?: ReactNode;
  description: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-1">
          <CardTitle className="flex items-center gap-1.5">
            {title}
            {help}
          </CardTitle>
          <CardDescription>{description}</CardDescription>
        </div>
        {action ? <div className="shrink-0">{action}</div> : null}
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  );
}

function IntegrationCard({
  routes,
  selectedRouteId,
  onSelectedRouteIdChange,
  openAiBaseUrl,
  anthropicBaseUrl,
  hasRuntimeKey,
  workspaceSlug,
  providers,
  agents,
}: {
  routes: GatewayRoute[];
  selectedRouteId: string;
  onSelectedRouteIdChange: (routeId: string) => void;
  openAiBaseUrl: string;
  anthropicBaseUrl: string;
  hasRuntimeKey: boolean;
  workspaceSlug: string;
  providers: ProviderConnection[];
  agents: Array<{ id: string; name: string }>;
}) {
  const hasRoutes = routes.length > 0;

  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
        <div className="grid gap-1">
          <CardDescription>Connect your app</CardDescription>
          <CardTitle>Send your app&apos;s traffic through this route</CardTitle>
        </div>
        {hasRoutes ? (
          <RouteSelector
            routes={routes}
            selectedRouteId={selectedRouteId}
            onSelectedRouteIdChange={onSelectedRouteIdChange}
          />
        ) : null}
      </CardHeader>
      <CardContent className="grid gap-4">
        {hasRoutes ? (
          <>
            {!hasRuntimeKey ? (
              <Alert>
                <IconAlertTriangle />
                <AlertTitle>You need an API key first</AlertTitle>
                <AlertDescription>
                  Put your gateway API key in the <code className="font-mono">TLG_API_KEY</code>{' '}
                  setting before running the examples below.
                  <Button asChild variant="link" className="h-auto px-1 py-0">
                    <Link href={`/api-keys?workspace=${encodeURIComponent(workspaceSlug)}`}>
                      Create an API key.
                    </Link>
                  </Button>
                </AlertDescription>
              </Alert>
            ) : null}
            <p className="text-sm text-muted-foreground">
              Point your existing AI client at the address below and use your gateway API key.
              That&apos;s the only change — every request is checked automatically from then on.
            </p>
            <Separator />
            <div className="grid gap-4 lg:grid-cols-2">
              <Snippet
                title="OpenAI-compatible"
                code={`import OpenAI from "openai";

const openai = new OpenAI({
  apiKey: process.env.TLG_API_KEY,
  baseURL: "${openAiBaseUrl}",
});

const response = await openai.chat.completions.create({
  model: "gpt-4o-mini",
  messages: [{ role: "user", content: userMessage }],
  max_tokens: 512,
});`}
              />
              <Snippet
                title="Anthropic"
                code={`import Anthropic from "@anthropic-ai/sdk";

const anthropic = new Anthropic({
  authToken: process.env.TLG_API_KEY,
  baseURL: "${anthropicBaseUrl}",
});

const response = await anthropic.messages.create({
  model: "claude-3-5-sonnet-latest",
  max_tokens: 512,
  messages: [{ role: "user", content: userMessage }],
});`}
              />
            </div>
          </>
        ) : (
          <EmptyState
            icon={<IconRoute />}
            title="No route to connect to yet"
            description="Create a route first. We'll then show ready-to-copy examples for OpenAI and Anthropic so you can connect your app in a couple of lines."
            action={
              <GatewayRouteDialog
                workspaceSlug={workspaceSlug}
                providers={providers}
                agents={agents}
              />
            }
          />
        )}
      </CardContent>
    </Card>
  );
}

function RouteSelector({
  routes,
  selectedRouteId,
  onSelectedRouteIdChange,
}: {
  routes: GatewayRoute[];
  selectedRouteId: string;
  onSelectedRouteIdChange: (routeId: string) => void;
}) {
  if (routes.length === 0) return null;
  return (
    <Select value={selectedRouteId} onValueChange={onSelectedRouteIdChange}>
      <SelectTrigger aria-label="Select gateway route" className="w-full md:w-[260px]">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {routes.map((route) => (
          <SelectItem key={route.id} value={route.id}>
            {route.display_name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

function Snippet({ title, code }: { title: string; code: string }) {
  const [copied, setCopied] = useState(false);
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (resetTimer.current) clearTimeout(resetTimer.current);
    },
    [],
  );

  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
    } catch {
      toast.error('Could not copy to clipboard');
      return;
    }
    setCopied(true);
    toast.success('Snippet copied');
    if (resetTimer.current) clearTimeout(resetTimer.current);
    resetTimer.current = setTimeout(() => setCopied(false), 1600);
  }

  return (
    <div className="min-w-0 overflow-hidden rounded-lg border bg-secondary">
      <div className="flex items-center justify-between gap-3 border-b px-3 py-2">
        <h3 className="font-mono text-xs font-medium text-muted-foreground">{title}</h3>
        <Button type="button" variant="ghost" size="sm" onClick={copy}>
          {copied ? <IconCircleCheck /> : <IconCopy />}
          {copied ? 'Copied' : 'Copy'}
        </Button>
      </div>
      <pre className="max-h-[360px] overflow-auto p-4 font-mono text-xs leading-relaxed text-foreground">
        <code>{code}</code>
      </pre>
    </div>
  );
}

// A compact one-tap copy affordance for inline tokens (like a route address).
// Mirrors the snippet copy pattern: writes to the clipboard and toasts on
// success or failure, so the operator gets feedback either way. Icon-only but
// keyboard-accessible via its descriptive aria-label.
function CopyButton({
  value,
  label,
  successMessage,
}: {
  value: string;
  label: string;
  successMessage: string;
}) {
  const [copied, setCopied] = useState(false);
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (resetTimer.current) clearTimeout(resetTimer.current);
    },
    [],
  );

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      toast.error('Could not copy to clipboard');
      return;
    }
    setCopied(true);
    toast.success(successMessage);
    if (resetTimer.current) clearTimeout(resetTimer.current);
    resetTimer.current = setTimeout(() => setCopied(false), 1600);
  }

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-xs"
      aria-label={label}
      onClick={copy}
      className="shrink-0 text-muted-foreground hover:text-foreground"
    >
      {copied ? <IconCircleCheck /> : <IconCopy />}
    </Button>
  );
}

function CredentialBadge({ status }: { status: string | null | undefined }) {
  const normalized = status?.trim().toLowerCase();
  if (normalized === 'sealed' || normalized === 'valid' || normalized === 'active') {
    return (
      <Badge variant="allow">
        <IconCircleCheck />
        Saved securely
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="text-[0.6875rem]">
      {normalized === 'missing' ? 'Not set' : titleize(status)}
    </Badge>
  );
}

function Field({
  label,
  htmlFor,
  optional,
  hint,
  children,
}: {
  label: string;
  htmlFor: string;
  optional?: boolean;
  hint?: ReactNode;
  children: ReactNode;
}) {
  const hintId = hint ? `${htmlFor}-hint` : undefined;
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={htmlFor}>
        {label}
        {optional ? <span className="ml-1 text-muted-foreground">(optional)</span> : null}
      </Label>
      {hint ? (
        <p id={hintId} className="text-xs leading-relaxed text-muted-foreground">
          {hint}
        </p>
      ) : null}
      {children}
    </div>
  );
}

function EntitySelect({
  label,
  id,
  value,
  values,
  hint,
  onValueChange,
}: {
  label: string;
  id: string;
  value: string;
  values: Array<{ id: string; label?: string; name?: string }>;
  hint?: ReactNode;
  onValueChange: (value: string) => void;
}) {
  return (
    <Field label={label} htmlFor={id} hint={hint}>
      <Select value={value} onValueChange={onValueChange} disabled={values.length === 0}>
        <SelectTrigger id={id} className="w-full">
          <SelectValue placeholder="Select..." />
        </SelectTrigger>
        <SelectContent>
          {values.map((item) => (
            <SelectItem key={item.id} value={item.id}>
              {item.label ?? item.name ?? item.id}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
}

async function sendGatewayConfig(
  url: string,
  method: 'POST' | 'PATCH',
  body: Record<string, string | number>,
): Promise<void> {
  const res = await fetch(url, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(readErrorMessage(text) ?? `request failed (${res.status})`);
  }
}

async function postGatewayConfig(
  url: string,
  body: Record<string, string | number>,
): Promise<void> {
  return sendGatewayConfig(url, 'POST', body);
}

async function deleteGatewayConfig(url: string): Promise<void> {
  const res = await fetch(url, { method: 'DELETE' });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(readErrorMessage(text) ?? `request failed (${res.status})`);
  }
}

function routeReadiness(data: GatewayPageData, route: GatewayRoute): RouteReadiness {
  if (!data.providerConnections.some((provider) => provider.id === route.provider_connection_id)) {
    return { label: 'Needs a provider', tone: 'warning' };
  }
  if (!data.agents.some((agent) => agent.id === route.agent_id)) {
    return { label: 'Needs an agent', tone: 'warning' };
  }
  if (data.activeRuntimeKeyCount === 0) {
    return { label: 'Needs an API key', tone: 'warning' };
  }
  return { label: 'Ready', tone: 'ready' };
}

function nameFor(rows: Array<{ id: string; display_name: string }>, id: string): string {
  return rows.find((row) => row.id === id)?.display_name ?? id;
}

function nameForAgent(rows: Array<{ id: string; name: string }>, id: string): string {
  return rows.find((row) => row.id === id)?.name ?? id;
}

function providerKindLabel(kind: GatewayProviderKind): string {
  if (kind === 'openai_compatible') return 'OpenAI-compatible';
  if (kind === 'anthropic') return 'Anthropic';
  return 'Payment HTTP';
}

// A column header with an inline "?" that defines the term in plain language.
function HeaderHint({ label, children }: { label: string; children: ReactNode }) {
  return (
    <span className="inline-flex items-center gap-1">
      {label}
      <InfoHint>{children}</InfoHint>
    </span>
  );
}

function titleize(value: string | null | undefined): string {
  const normalized = value?.trim();
  if (!normalized) return 'Unknown';

  return normalized
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function query(workspaceSlug: string): string {
  return `?workspace=${encodeURIComponent(workspaceSlug)}`;
}

function readErrorMessage(text: string): string | null {
  try {
    const parsed = JSON.parse(text) as { error?: string; message?: string };
    return parsed.message ?? parsed.error ?? null;
  } catch {
    return text.length > 0 ? text : null;
  }
}
