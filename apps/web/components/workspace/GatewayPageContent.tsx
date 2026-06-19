'use client';

import {
  IconAlertTriangle,
  IconCircleCheck,
  IconCopy,
  IconPlugConnected,
  IconPlus,
  IconRoute,
  IconShieldCheck,
} from '@tabler/icons-react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useState, type FormEvent, type ReactNode } from 'react';
import { toast } from 'sonner';
import type {
  EnforcementProfile,
  FailMode,
  GatewayInputAction,
  GatewayOutputAction,
  GatewayProviderConnection,
  GatewayProviderKind,
  GatewayRoute,
  ResponseMode,
  RetentionMode,
} from '@trustloopguard/sdk';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
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
import { Textarea } from '@/components/ui/textarea';
import type { DashboardShellData } from '@/lib/server/dashboard-data';

type ProviderConnection = GatewayProviderConnection;

type GatewayPageData = DashboardShellData & {
  providerConnections: ProviderConnection[];
  enforcementProfiles: EnforcementProfile[];
  gatewayRoutes: GatewayRoute[];
  activeRuntimeKeyCount: number;
};

type RouteReadiness = {
  label: string;
  tone: 'ready' | 'warning';
};

const INPUT_ACTIONS: GatewayInputAction[] = ['allow', 'block', 'redact'];
const OUTPUT_ACTIONS: GatewayOutputAction[] = ['block', 'rewrite', 'escalate', 'allow'];
const FAIL_MODES: FailMode[] = ['closed', 'open'];
const RETENTION_MODES: RetentionMode[] = ['metadata_only', 'redacted_body', 'full_body'];
const RESPONSE_MODES: ResponseMode[] = ['regular', 'streaming'];

const VERDICT_VARIANTS: ReadonlySet<string> = new Set(['allow', 'rewrite', 'block', 'escalate']);
type VerdictVariant = 'allow' | 'rewrite' | 'block' | 'escalate';

export function GatewayPageContent({
  data,
  apiBaseUrl,
}: {
  data: GatewayPageData;
  apiBaseUrl: string;
}) {
  const [selectedRouteId, setSelectedRouteId] = useState(data.gatewayRoutes[0]?.id ?? '');

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
      header: 'Provider',
      cell: (row) => providerKindLabel(row.kind),
    },
    {
      id: 'model',
      header: 'Default model',
      cell: (row) => <span className="font-mono text-xs">{row.default_model}</span>,
    },
    {
      id: 'credential',
      header: 'Credential',
      align: 'right',
      cell: (row) => <CredentialBadge status={row.credential_status} />,
    },
  ];

  const profileColumns: DataTableColumn<EnforcementProfile>[] = [
    {
      id: 'name',
      header: 'Name',
      cell: (row) => <span className="font-medium text-foreground">{row.display_name}</span>,
    },
    { id: 'input', header: 'Input', cell: (row) => <ActionBadge action={row.input_action} /> },
    { id: 'output', header: 'Output', cell: (row) => <ActionBadge action={row.output_action} /> },
    {
      id: 'failMode',
      header: 'Fail mode',
      cell: (row) => <span className="text-sm">{titleize(row.fail_mode)}</span>,
    },
    {
      id: 'retention',
      header: 'Retention',
      cell: (row) => <span className="text-sm">{titleize(row.retention_mode)}</span>,
    },
    {
      id: 'responseMode',
      header: 'Response',
      align: 'right',
      cell: (row) => <span className="text-sm">{titleize(row.response_mode)}</span>,
    },
  ];

  const routeColumns: DataTableColumn<GatewayRoute>[] = [
    {
      id: 'route',
      header: 'Route',
      cell: (row) => (
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate font-medium text-foreground">{row.display_name}</span>
          <span className="truncate font-mono text-xs text-muted-foreground">{row.id}</span>
        </div>
      ),
    },
    {
      id: 'provider',
      header: 'Provider',
      cell: (row) => (
        <span className="text-sm">{nameFor(data.providerConnections, row.provider_connection_id)}</span>
      ),
    },
    {
      id: 'agent',
      header: 'Agent',
      cell: (row) => <span className="text-sm">{nameForAgent(data.agents, row.agent_id)}</span>,
    },
    {
      id: 'profile',
      header: 'Profile',
      cell: (row) => (
        <span className="text-sm">{nameFor(data.enforcementProfiles, row.enforcement_profile_id)}</span>
      ),
    },
    {
      id: 'status',
      header: 'Status',
      align: 'right',
      cell: (row) => {
        const readiness = routeReadiness(data, row);
        return (
          <Badge variant={readiness.tone === 'ready' ? 'allow' : 'escalate'}>
            {readiness.tone === 'ready' ? (
              <IconCircleCheck />
            ) : (
              <IconAlertTriangle />
            )}
            {readiness.label}
          </Badge>
        );
      },
    },
  ];

  const providerCount = data.providerConnections.length;
  const profileCount = data.enforcementProfiles.length;
  const routeCount = data.gatewayRoutes.length;
  const hasRuntimeKey = data.activeRuntimeKeyCount > 0;
  const liveRoutes = data.gatewayRoutes.filter(
    (route) => routeReadiness(data, route).tone === 'ready',
  ).length;

  // Open on the first section that already has something to act on, so the
  // operator never lands on an empty tab when other tabs are populated.
  const defaultTab =
    routeCount > 0
      ? 'routes'
      : providerCount > 0
        ? 'providers'
        : profileCount > 0
          ? 'profiles'
          : 'routes';

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow={data.activeWorkspace.name}
        title="Gateway"
        description="Front model traffic with a guardrailed proxy. Bind a provider, an agent, and an enforcement profile into a drop-in OpenAI- or Anthropic-compatible endpoint."
        actions={
          <GatewayRouteDialog
            workspaceSlug={data.activeWorkspace.slug}
            providers={data.providerConnections}
            profiles={data.enforcementProfiles}
            agents={data.agents}
          />
        }
      />

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          icon={<IconRoute />}
          label="Gateway routes"
          value={routeCount}
          hint={routeCount > 0 ? `${liveRoutes} ready to serve` : 'None bound yet'}
        />
        <StatCard
          icon={<IconPlugConnected />}
          label="Provider connections"
          value={providerCount}
          hint={providerCount > 0 ? 'Credentials sealed by Rust' : 'No upstream yet'}
        />
        <StatCard
          icon={<IconShieldCheck />}
          label="Enforcement profiles"
          value={profileCount}
          hint={profileCount > 0 ? 'Govern input + output' : 'No behavior defined'}
        />
        <StatCard
          icon={hasRuntimeKey ? <IconCircleCheck /> : <IconAlertTriangle />}
          label="Runtime keys"
          value={data.activeRuntimeKeyCount}
          tone={hasRuntimeKey ? 'default' : 'warning'}
          hint={hasRuntimeKey ? 'Traffic can authenticate' : 'Required to serve traffic'}
        />
      </section>

      {!hasRuntimeKey ? (
        <Alert>
          <IconAlertTriangle />
          <AlertTitle>No runtime API key</AlertTitle>
          <AlertDescription>
            Gateway model traffic authenticates with a workspace runtime key.
            <Button asChild variant="link" className="h-auto px-1 py-0">
              <Link href={`/api-keys?workspace=${encodeURIComponent(data.activeWorkspace.slug)}`}>
                Create one before testing a route.
              </Link>
            </Button>
          </AlertDescription>
        </Alert>
      ) : null}

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
          <TabsTrigger value="profiles">
            Profiles
            <CountChip value={profileCount} />
          </TabsTrigger>
          <TabsTrigger value="integration">Integration</TabsTrigger>
        </TabsList>

        <TabsContent value="routes">
          <SectionCard
            title="Gateway routes"
            description="Each route binds a provider, an agent, and an enforcement profile into one callable endpoint."
            action={
              routeCount > 0 ? (
                <GatewayRouteDialog
                  workspaceSlug={data.activeWorkspace.slug}
                  providers={data.providerConnections}
                  profiles={data.enforcementProfiles}
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
                empty="No gateway routes yet."
              />
            ) : (
              <EmptyState
                icon={<IconRoute />}
                title="No gateway routes yet"
                description="A route is the endpoint your SDK points at. Add a provider connection and an enforcement profile first, then bind them into a route."
                action={
                  <GatewayRouteDialog
                    workspaceSlug={data.activeWorkspace.slug}
                    providers={data.providerConnections}
                    profiles={data.enforcementProfiles}
                    agents={data.agents}
                  />
                }
              />
            )}
          </SectionCard>
        </TabsContent>

        <TabsContent value="providers">
          <SectionCard
            title="Provider connections"
            description="Upstream model credentials. Keys are sealed by Rust on save and never returned to the dashboard."
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
                empty="No provider connections yet."
              />
            ) : (
              <EmptyState
                icon={<IconPlugConnected />}
                title="No provider connections yet"
                description="Connect an OpenAI-compatible or Anthropic upstream so the gateway has somewhere to forward traffic."
                action={<ProviderConnectionDialog workspaceSlug={data.activeWorkspace.slug} />}
              />
            )}
          </SectionCard>
        </TabsContent>

        <TabsContent value="profiles">
          <SectionCard
            title="Enforcement profiles"
            description="What the proxy does when a policy fires — on the way in and on the way out."
            action={
              profileCount > 0 ? (
                <EnforcementProfileDialog workspaceSlug={data.activeWorkspace.slug} />
              ) : null
            }
          >
            {profileCount > 0 ? (
              <DataTable
                columns={profileColumns}
                rows={data.enforcementProfiles}
                getRowKey={(profile) => profile.id}
                caption="Enforcement profiles in this workspace"
                empty="No enforcement profiles yet."
              />
            ) : (
              <EmptyState
                icon={<IconShieldCheck />}
                title="No enforcement profiles yet"
                description="Define how the gateway handles unsafe input and output — block, rewrite, redact, or escalate — before binding a route."
                action={<EnforcementProfileDialog workspaceSlug={data.activeWorkspace.slug} />}
              />
            )}
          </SectionCard>
        </TabsContent>

        <TabsContent value="integration">
          <IntegrationCard
            routes={data.gatewayRoutes}
            selectedRouteId={selectedRouteId}
            onSelectedRouteIdChange={setSelectedRouteId}
            openAiBaseUrl={openAiBaseUrl}
            anthropicBaseUrl={anthropicBaseUrl}
            hasRuntimeKey={hasRuntimeKey}
            workspaceSlug={data.activeWorkspace.slug}
            providers={data.providerConnections}
            profiles={data.enforcementProfiles}
            agents={data.agents}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
}

function ProviderConnectionDialog({ workspaceSlug }: { workspaceSlug: string }) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [id, setId] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [kind, setKind] = useState<GatewayProviderKind>('openai_compatible');
  const [baseUrl, setBaseUrl] = useState('');
  const [defaultModel, setDefaultModel] = useState('gpt-4o-mini');
  const [providerApiKey, setProviderApiKey] = useState('');

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    try {
      await postGatewayConfig(`/api/gateway/provider-connections${query(workspaceSlug)}`, {
        ...(id.trim() === '' ? {} : { id: id.trim() }),
        display_name: displayName.trim(),
        kind,
        ...(baseUrl.trim() === '' ? {} : { base_url: baseUrl.trim() }),
        default_model: defaultModel.trim(),
        provider_api_key: providerApiKey.trim(),
      });
      toast.success('Provider connection created');
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
    setKind('openai_compatible');
    setBaseUrl('');
    setDefaultModel('gpt-4o-mini');
    setProviderApiKey('');
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline">
          <IconPlus />
          Provider
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-lg">
        <form onSubmit={onSubmit} className="grid gap-4">
          <DialogHeader>
            <DialogTitle>Create provider connection</DialogTitle>
            <DialogDescription>
              Store a provider credential for gateway model traffic. The key is sealed on save and
              never shown again.
            </DialogDescription>
          </DialogHeader>
          <Field label="Route-safe id" htmlFor="provider-id" optional>
            <Input
              id="provider-id"
              autoComplete="off"
              className="font-mono"
              placeholder="openai-prod"
              value={id}
              onChange={(event) => setId(event.target.value)}
            />
          </Field>
          <Field label="Name" htmlFor="provider-name">
            <Input
              id="provider-name"
              required
              autoComplete="off"
              placeholder="OpenAI production"
              value={displayName}
              onChange={(event) => setDisplayName(event.target.value)}
            />
          </Field>
          <Field label="Provider" htmlFor="provider-kind">
            <Select
              value={kind}
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
              </SelectContent>
            </Select>
          </Field>
          <Field label="Base URL" htmlFor="provider-base-url" optional>
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
          <Field label="Default model" htmlFor="provider-model">
            <Input
              id="provider-model"
              required
              autoComplete="off"
              className="font-mono"
              value={defaultModel}
              onChange={(event) => setDefaultModel(event.target.value)}
            />
          </Field>
          <Field label="Provider API key" htmlFor="provider-key">
            <Input
              id="provider-key"
              required
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
                !defaultModel.trim() ||
                !providerApiKey.trim()
              }
            >
              {submitting ? 'Creating...' : 'Create provider'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function EnforcementProfileDialog({ workspaceSlug }: { workspaceSlug: string }) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [id, setId] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [inputAction, setInputAction] = useState<GatewayInputAction>('block');
  const [outputAction, setOutputAction] = useState<GatewayOutputAction>('rewrite');
  const [failMode, setFailMode] = useState<FailMode>('closed');
  const [retentionMode, setRetentionMode] = useState<RetentionMode>('metadata_only');
  const [responseMode, setResponseMode] = useState<ResponseMode>('regular');
  const [fallbackMessage, setFallbackMessage] = useState("I can't help with that request.");
  const [maxRegenerations, setMaxRegenerations] = useState('1');

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    try {
      await postGatewayConfig(`/api/enforcement-profiles${query(workspaceSlug)}`, {
        ...(id.trim() === '' ? {} : { id: id.trim() }),
        display_name: displayName.trim(),
        input_action: inputAction,
        output_action: outputAction,
        fail_mode: failMode,
        retention_mode: retentionMode,
        response_mode: responseMode,
        fallback_message: fallbackMessage.trim(),
        max_regenerations: Number.parseInt(maxRegenerations, 10),
      });
      toast.success('Enforcement profile created');
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
    setInputAction('block');
    setOutputAction('rewrite');
    setFailMode('closed');
    setRetentionMode('metadata_only');
    setResponseMode('regular');
    setFallbackMessage("I can't help with that request.");
    setMaxRegenerations('1');
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline">
          <IconPlus />
          Profile
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-xl">
        <form onSubmit={onSubmit} className="grid gap-4">
          <DialogHeader>
            <DialogTitle>Create enforcement profile</DialogTitle>
            <DialogDescription>
              Choose what the gateway does when a policy fires.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Route-safe id" htmlFor="profile-id" optional>
              <Input
                id="profile-id"
                autoComplete="off"
                className="font-mono"
                placeholder="strict-prod"
                value={id}
                onChange={(event) => setId(event.target.value)}
              />
            </Field>
            <Field label="Name" htmlFor="profile-name">
              <Input
                id="profile-name"
                required
                autoComplete="off"
                placeholder="Strict production"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
              />
            </Field>
            <EnumSelect
              label="Input action"
              id="profile-input-action"
              value={inputAction}
              values={INPUT_ACTIONS}
              onValueChange={setInputAction}
            />
            <EnumSelect
              label="Output action"
              id="profile-output-action"
              value={outputAction}
              values={OUTPUT_ACTIONS}
              onValueChange={setOutputAction}
            />
            <EnumSelect
              label="Fail mode"
              id="profile-fail-mode"
              value={failMode}
              values={FAIL_MODES}
              onValueChange={setFailMode}
            />
            <EnumSelect
              label="Retention"
              id="profile-retention"
              value={retentionMode}
              values={RETENTION_MODES}
              onValueChange={setRetentionMode}
            />
            <EnumSelect
              label="Response mode"
              id="profile-response-mode"
              value={responseMode}
              values={RESPONSE_MODES}
              onValueChange={setResponseMode}
            />
            <Field label="Max regenerations" htmlFor="profile-max-regenerations">
              <Input
                id="profile-max-regenerations"
                required
                type="number"
                min={0}
                max={5}
                className="tabular-nums"
                value={maxRegenerations}
                onChange={(event) => setMaxRegenerations(event.target.value)}
              />
            </Field>
          </div>
          <Field label="Fallback message" htmlFor="profile-fallback">
            <Textarea
              id="profile-fallback"
              required
              rows={3}
              value={fallbackMessage}
              onChange={(event) => setFallbackMessage(event.target.value)}
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
                !fallbackMessage.trim() ||
                Number.isNaN(Number.parseInt(maxRegenerations, 10))
              }
            >
              {submitting ? 'Creating...' : 'Create profile'}
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
  profiles,
  agents,
}: {
  workspaceSlug: string;
  providers: ProviderConnection[];
  profiles: EnforcementProfile[];
  agents: Array<{ id: string; name: string }>;
}) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [id, setId] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [providerId, setProviderId] = useState(providers[0]?.id ?? '');
  const [profileId, setProfileId] = useState(profiles[0]?.id ?? '');
  const [agentId, setAgentId] = useState(agents[0]?.id ?? '');

  useEffect(() => {
    setProviderId((current) => current || providers[0]?.id || '');
  }, [providers]);
  useEffect(() => {
    setProfileId((current) => current || profiles[0]?.id || '');
  }, [profiles]);
  useEffect(() => {
    setAgentId((current) => current || agents[0]?.id || '');
  }, [agents]);

  const hasDependencies = providers.length > 0 && profiles.length > 0 && agents.length > 0;

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
        enforcement_profile_id: profileId,
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
    setProviderId(providers[0]?.id ?? '');
    setProfileId(profiles[0]?.id ?? '');
    setAgentId(agents[0]?.id ?? '');
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
            <DialogTitle>Create gateway route</DialogTitle>
            <DialogDescription>
              Bind a provider, agent profile, and enforcement profile into a provider-compatible
              endpoint.
            </DialogDescription>
          </DialogHeader>
          {!hasDependencies ? (
            <Alert>
              <IconAlertTriangle />
              <AlertTitle>Route prerequisites missing</AlertTitle>
              <AlertDescription>
                Create at least one provider connection, enforcement profile, and agent first.
              </AlertDescription>
            </Alert>
          ) : null}
          <Field label="Route id" htmlFor="route-id" optional>
            <Input
              id="route-id"
              autoComplete="off"
              className="font-mono"
              placeholder="support-prod"
              value={id}
              onChange={(event) => setId(event.target.value)}
            />
          </Field>
          <Field label="Name" htmlFor="route-name">
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
            label="Provider connection"
            id="route-provider"
            value={providerId}
            values={providers.map((provider) => ({
              id: provider.id,
              label: provider.display_name,
            }))}
            onValueChange={setProviderId}
          />
          <EntitySelect
            label="Agent"
            id="route-agent"
            value={agentId}
            values={agents}
            onValueChange={setAgentId}
          />
          <EntitySelect
            label="Enforcement profile"
            id="route-profile"
            value={profileId}
            values={profiles.map((profile) => ({
              id: profile.id,
              label: profile.display_name,
            }))}
            onValueChange={setProfileId}
          />
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={
                submitting ||
                !hasDependencies ||
                !displayName.trim() ||
                !providerId ||
                !profileId ||
                !agentId
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
  description,
  action,
  children,
}: {
  title: string;
  description: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-1">
          <CardTitle>{title}</CardTitle>
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
  profiles,
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
  profiles: EnforcementProfile[];
  agents: Array<{ id: string; name: string }>;
}) {
  const hasRoutes = routes.length > 0;

  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
        <div className="grid gap-1">
          <CardDescription>Provider SDK integration</CardDescription>
          <CardTitle>Point model traffic at this route</CardTitle>
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
                <AlertTitle>Runtime key needed to authenticate</AlertTitle>
                <AlertDescription>
                  Set <code className="font-mono">TLG_API_KEY</code> to a workspace runtime key
                  before sending traffic through these snippets.
                </AlertDescription>
              </Alert>
            ) : null}
            <p className="text-sm text-muted-foreground">
              Swap your provider client&apos;s base URL for the gateway and authenticate with your
              workspace runtime key — guardrails apply with no other code changes.
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
            title="No route to integrate yet"
            description="Create a gateway route to generate ready-to-paste OpenAI- and Anthropic-compatible client snippets."
            action={
              <GatewayRouteDialog
                workspaceSlug={workspaceSlug}
                providers={providers}
                profiles={profiles}
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
      <SelectTrigger className="w-full md:w-[260px]">
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

  async function copy() {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    toast.success('Snippet copied');
    window.setTimeout(() => setCopied(false), 1600);
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

function CredentialBadge({ status }: { status: string | null | undefined }) {
  const normalized = status?.trim().toLowerCase();
  if (normalized === 'sealed' || normalized === 'valid' || normalized === 'active') {
    return (
      <Badge variant="allow">
        <IconCircleCheck />
        {titleize(status)}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="font-mono text-[0.6875rem]">
      {titleize(status)}
    </Badge>
  );
}

function ActionBadge({ action }: { action: string | null | undefined }) {
  const key = action?.trim().toLowerCase();
  if (key && VERDICT_VARIANTS.has(key)) {
    return (
      <Badge variant={key as VerdictVariant} className="font-mono text-[0.6875rem]">
        {titleize(action)}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="font-mono text-[0.6875rem]">
      {titleize(action)}
    </Badge>
  );
}

function Field({
  label,
  htmlFor,
  optional,
  children,
}: {
  label: string;
  htmlFor: string;
  optional?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>
        {label}
        {optional ? <span className="ml-1 text-muted-foreground">(optional)</span> : null}
      </Label>
      {children}
    </div>
  );
}

function EnumSelect<T extends string>({
  label,
  id,
  value,
  values,
  onValueChange,
}: {
  label: string;
  id: string;
  value: T;
  values: T[];
  onValueChange: (value: T) => void;
}) {
  return (
    <Field label={label} htmlFor={id}>
      <Select value={value} onValueChange={(next) => onValueChange(next as T)}>
        <SelectTrigger id={id} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {values.map((item) => (
            <SelectItem key={item} value={item}>
              {titleize(item)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
}

function EntitySelect({
  label,
  id,
  value,
  values,
  onValueChange,
}: {
  label: string;
  id: string;
  value: string;
  values: Array<{ id: string; label?: string; name?: string }>;
  onValueChange: (value: string) => void;
}) {
  return (
    <Field label={label} htmlFor={id}>
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

async function postGatewayConfig(url: string, body: Record<string, string | number>): Promise<void> {
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(readErrorMessage(text) ?? `request failed (${res.status})`);
  }
}

function routeReadiness(data: GatewayPageData, route: GatewayRoute): RouteReadiness {
  if (!data.providerConnections.some((provider) => provider.id === route.provider_connection_id)) {
    return { label: 'Missing provider', tone: 'warning' };
  }
  if (!data.enforcementProfiles.some((profile) => profile.id === route.enforcement_profile_id)) {
    return { label: 'Missing profile', tone: 'warning' };
  }
  if (!data.agents.some((agent) => agent.id === route.agent_id)) {
    return { label: 'Missing agent', tone: 'warning' };
  }
  if (data.activeRuntimeKeyCount === 0) {
    return { label: 'No runtime key', tone: 'warning' };
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
  return kind === 'openai_compatible' ? 'OpenAI-compatible' : 'Anthropic';
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
