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
import { useEffect, useRef, useState, type FormEvent, type ReactNode } from 'react';
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
  ];

  const profileColumns: DataTableColumn<EnforcementProfile>[] = [
    {
      id: 'name',
      header: 'Name',
      cell: (row) => <span className="font-medium text-foreground">{row.display_name}</span>,
    },
    {
      id: 'input',
      header: (
        <HeaderHint label="On the way in">
          What to do when something risky is sent to the AI — let it through, block it, or hide the
          sensitive parts.
        </HeaderHint>
      ),
      cell: (row) => <ActionBadge action={row.input_action} />,
    },
    {
      id: 'output',
      header: (
        <HeaderHint label="On the way back">
          What to do when the AI&apos;s reply looks risky — let it through, block it, clean it up, or
          send it for a person to review.
        </HeaderHint>
      ),
      cell: (row) => <ActionBadge action={row.output_action} />,
    },
    {
      id: 'failMode',
      header: (
        <HeaderHint label="If a check fails">
          What happens if a safety check can&apos;t run — block the request to stay safe, or let it
          through to avoid interruptions.
        </HeaderHint>
      ),
      cell: (row) => <span className="text-sm">{friendlyLabel(row.fail_mode)}</span>,
    },
    {
      id: 'retention',
      header: (
        <HeaderHint label="What we keep">
          How much of each request and reply is stored for your records — just the summary, a
          redacted copy, or the full text.
        </HeaderHint>
      ),
      cell: (row) => <span className="text-sm">{friendlyLabel(row.retention_mode)}</span>,
    },
    {
      id: 'responseMode',
      header: 'Delivery',
      align: 'right',
      cell: (row) => <span className="text-sm">{friendlyLabel(row.response_mode)}</span>,
    },
  ];

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
        <span className="text-sm">{nameFor(data.providerConnections, row.provider_connection_id)}</span>
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
      id: 'profile',
      header: (
        <HeaderHint label="Rule set">
          The set of rules that decides what happens to risky requests on this route.
        </HeaderHint>
      ),
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
        help={<InfoHint term="gateway" />}
        description="Route your AI provider calls through TrustLoopGuard so every request is checked automatically, with no code changes."
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
          icon={<IconShieldCheck />}
          label="Rule sets"
          value={profileCount}
          hint={profileCount > 0 ? 'Checking requests' : 'None defined yet'}
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
            Rule sets
            <CountChip value={profileCount} />
          </TabsTrigger>
          <TabsTrigger value="integration">Connect your app</TabsTrigger>
        </TabsList>

        <TabsContent value="routes">
          <SectionCard
            title="Routes"
            help={<InfoHint term="route" />}
            description="A route is the address your app sends traffic to. It ties together one provider, one agent, and one set of rules."
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
                empty="No routes yet."
              />
            ) : (
              <EmptyState
                icon={<IconRoute />}
                title="No routes set up yet"
                description="A route is the address your app points at. Connect a provider and pick a rule set first, then create a route to start sending traffic through the checks."
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

        <TabsContent value="profiles">
          <SectionCard
            title="Rule sets"
            help={<InfoHint term="enforcementProfile" />}
            description="A reusable set of rules that decides what happens when a request looks risky — on the way in and on the way back. Apply the same set to as many routes as you like."
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
                empty="No rule sets yet."
              />
            ) : (
              <EmptyState
                icon={<IconShieldCheck />}
                title="No rule sets yet"
                description="Decide what the gateway should do with risky requests — allow, block, clean them up, or send them for a person to review — then reuse that set on any route."
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
            <DialogTitle>Connect a provider</DialogTitle>
            <DialogDescription>
              Connect an AI service like OpenAI or Anthropic so the gateway has somewhere to send
              your requests. Your secret key is stored securely and never shown again.
            </DialogDescription>
          </DialogHeader>
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
          <Field
            label="Service address"
            htmlFor="provider-base-url"
            optional
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
          <Field
            label="Secret key"
            htmlFor="provider-key"
            hint="The API key from your AI service account. We store it securely and never show it again."
          >
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
              {submitting ? 'Connecting...' : 'Connect provider'}
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
            <DialogTitle>Create a rule set</DialogTitle>
            <DialogDescription>
              A rule set decides what happens when a request looks risky. Set it once, then reuse it
              on any route. You can change these settings later.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field
              label="Short id"
              htmlFor="profile-id"
              optional
              hint="A short, lowercase nickname used in links. Leave blank to generate one."
            >
              <Input
                id="profile-id"
                autoComplete="off"
                className="font-mono"
                placeholder="strict-prod"
                value={id}
                onChange={(event) => setId(event.target.value)}
              />
            </Field>
            <Field
              label="Name"
              htmlFor="profile-name"
              hint="A friendly name you'll recognize in lists."
            >
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
              label="On the way in"
              id="profile-input-action"
              value={inputAction}
              values={INPUT_ACTIONS}
              hint="What to do with a risky request before it reaches the AI."
              onValueChange={setInputAction}
            />
            <EnumSelect
              label="On the way back"
              id="profile-output-action"
              value={outputAction}
              values={OUTPUT_ACTIONS}
              hint="What to do with a risky reply before it reaches your users."
              onValueChange={setOutputAction}
            />
            <EnumSelect
              label="If a check fails"
              id="profile-fail-mode"
              value={failMode}
              values={FAIL_MODES}
              hint="What happens if a safety check can't run. Block to stay safe, or allow to avoid interruptions."
              onValueChange={setFailMode}
            />
            <EnumSelect
              label="What we keep"
              id="profile-retention"
              value={retentionMode}
              values={RETENTION_MODES}
              hint="How much of each request and reply is stored for your records."
              onValueChange={setRetentionMode}
            />
            <EnumSelect
              label="Delivery"
              id="profile-response-mode"
              value={responseMode}
              values={RESPONSE_MODES}
              hint="Send the full reply at once (standard), or word by word as it's written (streaming)."
              onValueChange={setResponseMode}
            />
            <Field
              label="Retry limit"
              htmlFor="profile-max-regenerations"
              hint="How many times the AI may try again to produce a safe reply (0–5)."
            >
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
          <Field
            label="Blocked-request message"
            htmlFor="profile-fallback"
            hint="What your users see when a request is blocked. Keep it polite and clear."
          >
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
              {submitting ? 'Creating...' : 'Create rule set'}
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
            <DialogTitle>Create a route</DialogTitle>
            <DialogDescription>
              A route is the address your app points at. Pick one provider, one agent, and one rule
              set, and the gateway checks every request that flows through it.
            </DialogDescription>
          </DialogHeader>
          {!hasDependencies ? (
            <Alert>
              <IconAlertTriangle />
              <AlertTitle>A few things are needed first</AlertTitle>
              <AlertDescription>
                Connect at least one provider, create a rule set, and add an agent before you can
                build a route.
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
          <EntitySelect
            label="Rule set"
            id="route-profile"
            value={profileId}
            values={profiles.map((profile) => ({
              id: profile.id,
              label: profile.display_name,
            }))}
            hint="The rules that decide what happens to risky requests on this route."
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

function ActionBadge({ action }: { action: string | null | undefined }) {
  const key = action?.trim().toLowerCase();
  if (key && VERDICT_VARIANTS.has(key)) {
    return (
      <Badge variant={key as VerdictVariant} className="text-[0.6875rem]">
        {friendlyLabel(action)}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="text-[0.6875rem]">
      {friendlyLabel(action)}
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

function EnumSelect<T extends string>({
  label,
  id,
  value,
  values,
  hint,
  onValueChange,
}: {
  label: string;
  id: string;
  value: T;
  values: T[];
  hint?: ReactNode;
  onValueChange: (value: T) => void;
}) {
  return (
    <Field label={label} htmlFor={id} hint={hint}>
      <Select value={value} onValueChange={(next) => onValueChange(next as T)}>
        <SelectTrigger id={id} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {values.map((item) => (
            <SelectItem key={item} value={item}>
              {friendlyLabel(item)}
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
    return { label: 'Needs a provider', tone: 'warning' };
  }
  if (!data.enforcementProfiles.some((profile) => profile.id === route.enforcement_profile_id)) {
    return { label: 'Needs a rule set', tone: 'warning' };
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
  return kind === 'openai_compatible' ? 'OpenAI-compatible' : 'Anthropic';
}

// Plain-language wording for the raw enum values so a non-technical reader never
// has to decode a value like `metadata_only` or `closed`. Falls back to the
// title-cased value if an unexpected enum slips through.
const FRIENDLY_LABELS: Record<string, string> = {
  // Input actions
  allow: 'Allow',
  block: 'Block',
  redact: 'Hide sensitive parts',
  // Output actions
  rewrite: 'Rewrite',
  escalate: 'Send for review',
  // Fail modes
  closed: 'Block when unsure',
  open: 'Allow when unsure',
  // Retention modes
  metadata_only: 'Metadata only',
  redacted_body: 'Redacted text',
  full_body: 'Full text',
  // Response modes
  regular: 'Standard',
  streaming: 'Streaming',
};

function friendlyLabel(value: string | null | undefined): string {
  const key = value?.trim().toLowerCase();
  if (key && FRIENDLY_LABELS[key]) return FRIENDLY_LABELS[key];
  return titleize(value);
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
