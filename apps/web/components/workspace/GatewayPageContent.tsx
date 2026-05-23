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
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
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
    { id: 'name', header: 'Name', cell: (row) => row.display_name },
    {
      id: 'kind',
      header: 'Provider',
      cell: (row) => providerKindLabel(row.kind),
    },
    {
      id: 'model',
      header: 'Default model',
      cell: (row) => row.default_model,
      cellClassName: 'font-mono text-xs',
    },
    {
      id: 'credential',
      header: 'Credential',
      cell: (row) => (
        <Badge variant="outline" className="rounded-sm">
          {titleize(row.credential_status)}
        </Badge>
      ),
    },
  ];

  const profileColumns: DataTableColumn<EnforcementProfile>[] = [
    { id: 'name', header: 'Name', cell: (row) => row.display_name },
    { id: 'input', header: 'Input', cell: (row) => titleize(row.input_action) },
    { id: 'output', header: 'Output', cell: (row) => titleize(row.output_action) },
    { id: 'failMode', header: 'Fail mode', cell: (row) => titleize(row.fail_mode) },
    { id: 'retention', header: 'Retention', cell: (row) => titleize(row.retention_mode) },
  ];

  const routeColumns: DataTableColumn<GatewayRoute>[] = [
    {
      id: 'route',
      header: 'Route',
      cell: (row) => row.display_name,
    },
    {
      id: 'provider',
      header: 'Provider',
      cell: (row) => nameFor(data.providerConnections, row.provider_connection_id),
    },
    {
      id: 'agent',
      header: 'Agent',
      cell: (row) => nameForAgent(data.agents, row.agent_id),
    },
    {
      id: 'profile',
      header: 'Profile',
      cell: (row) => nameFor(data.enforcementProfiles, row.enforcement_profile_id),
    },
    {
      id: 'status',
      header: 'Status',
      cell: (row) => {
        const readiness = routeReadiness(data, row);
        return (
          <Badge
            variant="outline"
            className={
              readiness.tone === 'ready'
                ? 'rounded-sm border-emerald-200 bg-emerald-50 text-emerald-700'
                : 'rounded-sm border-amber-200 bg-amber-50 text-amber-700'
            }
          >
            {readiness.label}
          </Badge>
        );
      },
    },
  ];

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">{data.activeWorkspace.name}</p>
          <h2 className="text-2xl font-semibold">Gateway</h2>
        </div>
        <div className="flex flex-wrap gap-2">
          <ProviderConnectionDialog workspaceSlug={data.activeWorkspace.slug} />
          <EnforcementProfileDialog workspaceSlug={data.activeWorkspace.slug} />
          <GatewayRouteDialog
            workspaceSlug={data.activeWorkspace.slug}
            providers={data.providerConnections}
            profiles={data.enforcementProfiles}
            agents={data.agents}
          />
        </div>
      </div>

      <section className="grid gap-4 md:grid-cols-4">
        <SummaryCard
          icon={<IconPlugConnected />}
          label="Provider connections"
          value={data.providerConnections.length}
        />
        <SummaryCard
          icon={<IconShieldCheck />}
          label="Enforcement profiles"
          value={data.enforcementProfiles.length}
        />
        <SummaryCard icon={<IconRoute />} label="Gateway routes" value={data.gatewayRoutes.length} />
        <SummaryCard
          icon={data.activeRuntimeKeyCount > 0 ? <IconCircleCheck /> : <IconAlertTriangle />}
          label="Runtime keys"
          value={data.activeRuntimeKeyCount}
        />
      </section>

      {data.activeRuntimeKeyCount === 0 ? (
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

      <div className="grid gap-4 xl:grid-cols-3">
        <ConfigCard
          title="Provider connections"
          description="Provider credentials are sealed by Rust and never returned."
        >
          <DataTable
            columns={providerColumns}
            rows={data.providerConnections}
            getRowKey={(provider) => provider.id}
            empty="No provider connections yet."
          />
        </ConfigCard>

        <ConfigCard
          title="Enforcement profiles"
          description="Profiles decide how the proxy handles unsafe input and output."
        >
          <DataTable
            columns={profileColumns}
            rows={data.enforcementProfiles}
            getRowKey={(profile) => profile.id}
            empty="No enforcement profiles yet."
          />
        </ConfigCard>

        <ConfigCard
          title="Gateway routes"
          description="Routes bind provider, agent, and enforcement behavior."
        >
          <DataTable
            columns={routeColumns}
            rows={data.gatewayRoutes}
            getRowKey={(route) => route.id}
            empty="No gateway routes yet."
          />
        </ConfigCard>
      </div>

      <Card>
        <CardHeader className="gap-3 md:flex-row md:items-start md:justify-between">
          <div>
            <CardDescription>Provider SDK integration</CardDescription>
            <CardTitle>Point model traffic at this route</CardTitle>
          </div>
          <RouteSelector
            routes={data.gatewayRoutes}
            selectedRouteId={selectedRouteId}
            onSelectedRouteIdChange={setSelectedRouteId}
          />
        </CardHeader>
        <CardContent className="grid gap-4 lg:grid-cols-2">
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
        </CardContent>
      </Card>
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
              Store a provider credential for gateway model traffic.
            </DialogDescription>
          </DialogHeader>
          <Field label="Route-safe id" htmlFor="provider-id" optional>
            <Input
              id="provider-id"
              autoComplete="off"
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
            <Field label="Max regenerations" htmlFor="profile-max-regenerations">
              <Input
                id="profile-max-regenerations"
                required
                type="number"
                min={0}
                max={5}
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
          Route
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

function SummaryCard({ icon, label, value }: { icon: ReactNode; label: string; value: number }) {
  return (
    <Card>
      <CardContent className="flex items-center gap-3 p-4">
        <div className="flex size-9 items-center justify-center rounded-md bg-muted text-muted-foreground">
          {icon}
        </div>
        <div>
          <div className="text-xs font-medium uppercase text-muted-foreground">{label}</div>
          <div className="text-2xl font-semibold">{value}</div>
        </div>
      </CardContent>
    </Card>
  );
}

function ConfigCard({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <Card>
      <CardHeader>
        <CardDescription>{description}</CardDescription>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-x-auto">{children}</div>
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
  async function copy() {
    await navigator.clipboard.writeText(code);
    toast.success('Snippet copied');
  }

  return (
    <div className="min-w-0">
      <div className="mb-2 flex items-center justify-between gap-3">
        <h3 className="text-sm font-medium">{title}</h3>
        <Button type="button" variant="outline" size="sm" onClick={copy}>
          <IconCopy />
          Copy
        </Button>
      </div>
      <pre className="max-h-[360px] overflow-auto rounded-lg bg-slate-950 p-4 text-xs text-slate-50">
        <code>{code}</code>
      </pre>
    </div>
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

function titleize(value: string): string {
  return value
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
