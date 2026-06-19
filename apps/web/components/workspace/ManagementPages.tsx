import Link from 'next/link';
import {
  IconArrowRight,
  IconBell,
  IconBook2,
  IconBuilding,
  IconExternalLink,
  IconGauge,
  IconId,
  IconLayoutGrid,
  IconPlus,
  IconRobot,
  IconSettings,
  IconShieldCheck,
  IconUserCircle,
  IconUsers,
  IconWebhook,
  type Icon,
} from '@tabler/icons-react';
import type { ReactNode } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { InviteMemberDialog } from '@/components/workspace/InviteMemberDialog';
import { KnowledgeSourceCreateDialog } from '@/components/workspace/KnowledgeSourceCreateDialog';
import { PendingInvitesTable } from '@/components/workspace/PendingInvitesTable';
import { QuickCreateAgentDialog } from '@/components/workspace/QuickCreateAgentDialog';
import { RunDetailLiveView } from '@/components/workspace/RunDetailLiveView';
import { RunsLiveTable } from '@/components/workspace/RunsLiveTable';
import { AnalyticsChartGrid } from '@/components/analytics/AnalyticsChartGrid';
import { cn } from '@/lib/utils';
import type { RunDetailSnapshot } from '@/lib/run-detail-live';
import type {
  AgentRow,
  AnalyticsCatalog,
  AnalyticsDashboardView,
  DashboardShellData,
  KnowledgeSourceRow,
  RunEventRow,
  RunRow,
  RunTraceRow,
  TeamInviteRow,
  TeamMemberRow,
  WorkspaceDashboardData,
} from '@/lib/server/dashboard-data';

export function WorkspacesPageContent({ data }: { data: DashboardShellData }) {
  return (
    <PageShell
      eyebrow={data.organization.name}
      title="Workspaces"
      description="Each workspace is an isolated set of policies, agents, and knowledge. Switch between them or spin up a new one."
      actionLabel="New workspace"
      actionHref="/onboarding/workspace"
      actionIcon={IconPlus}
    >
      {data.workspaces.length === 0 ? (
        <EmptyState
          icon={<IconLayoutGrid />}
          title="No workspaces yet"
          description="Create your first workspace to start configuring guardrail policies and connecting agents."
          action={
            <Button asChild>
              <Link href="/onboarding/workspace">
                <IconPlus />
                New workspace
              </Link>
            </Button>
          }
        />
      ) : (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {data.workspaces.map((workspace) => {
            const isActive = workspace.id === data.activeWorkspace.id;
            return (
              <Card
                key={workspace.id}
                className="group gap-0 overflow-hidden transition-colors hover:border-foreground/20"
              >
                <CardHeader className="pb-4">
                  <CardDescription className="flex items-center gap-1.5">
                    <IconBuilding className="size-3.5" />
                    {workspace.role}
                  </CardDescription>
                  <CardTitle className="truncate text-base" title={workspace.name}>
                    {workspace.name}
                  </CardTitle>
                  <CardAction>
                    {isActive ? (
                      <Badge variant="secondary" className="font-data text-xs">
                        active
                      </Badge>
                    ) : null}
                  </CardAction>
                </CardHeader>
                <CardContent className="grid gap-4">
                  <p className="line-clamp-2 min-h-[2.5rem] text-sm text-muted-foreground">
                    {workspace.description || 'No description provided.'}
                  </p>
                  <div className="grid grid-cols-3 gap-2">
                    <Stat label="Policies" value={String(workspace.policyCount)} />
                    <Stat label="Agents" value={String(workspace.agentCount)} />
                    <Stat label="Sources" value={String(workspace.sourceCount)} />
                  </div>
                </CardContent>
                <CardFooter className="mt-4 border-t pt-4">
                  <Button
                    asChild
                    variant={isActive ? 'outline' : 'ghost'}
                    size="sm"
                    className="w-full justify-between"
                  >
                    <Link href={`/?workspace=${workspace.slug}`}>
                      {isActive ? 'Current workspace' : 'Open workspace'}
                      <IconArrowRight className="transition-transform group-hover:translate-x-0.5 motion-reduce:transition-none" />
                    </Link>
                  </Button>
                </CardFooter>
              </Card>
            );
          })}
        </div>
      )}
    </PageShell>
  );
}

const agentColumns: DataTableColumn<AgentRow>[] = [
  { id: 'name', header: 'Agent', cell: (row) => <span className="font-medium">{row.name}</span> },
  {
    id: 'scope',
    header: 'Scope',
    cell: (row) => row.scope,
    cellClassName: 'text-muted-foreground',
  },
  {
    id: 'policies',
    header: 'Policies',
    align: 'right',
    cell: (row) => row.policies,
    cellClassName: 'font-data text-xs',
  },
  {
    id: 'status',
    header: 'Status',
    align: 'right',
    cell: (row) => (
      <Badge variant="outline" className="font-data text-xs">
        {row.status}
      </Badge>
    ),
  },
];

export function AgentsPageContent({
  data,
}: {
  data: DashboardShellData & { agents: AgentRow[] };
}) {
  return (
    <PageShell
      eyebrow={data.activeWorkspace.name}
      title="Agents"
      description="Agent profiles describe which policies guard each of your AI agents at runtime."
      action={
        <QuickCreateAgentDialog>
          <Button>
            <IconRobot />
            New agent
          </Button>
        </QuickCreateAgentDialog>
      }
    >
      <SectionCard
        icon={IconRobot}
        eyebrow="Workspace-owned agent profiles"
        title="Guardrail agents"
        count={data.agents.length}
      >
        {data.agents.length === 0 ? (
          <EmptyState
            icon={<IconRobot />}
            title="No agents in this workspace yet"
            description="Register an agent profile to attach guardrail policies and start checking its traffic."
            action={
              <QuickCreateAgentDialog>
                <Button size="sm" variant="outline">
                  <IconRobot />
                  New agent
                </Button>
              </QuickCreateAgentDialog>
            }
          />
        ) : (
          <DataTable
            columns={agentColumns}
            rows={data.agents}
            getRowKey={(agent) => agent.id}
            caption="Guardrail agent profiles"
            empty="No agents in this workspace yet."
          />
        )}
      </SectionCard>
    </PageShell>
  );
}

export function RunsPageContent({
  data,
}: {
  data: DashboardShellData & { runs: RunRow[] };
}) {
  return (
    <PageShell
      eyebrow={data.activeWorkspace.name}
      title="Runs"
      description="Every guardrail check streamed from your agents, newest first."
    >
      <RunsLiveTable initialRuns={data.runs} workspaceSlug={data.activeWorkspace.slug} />
    </PageShell>
  );
}

export function AnalyticsPageContent({
  data,
}: {
  data: DashboardShellData & {
    analyticsCatalog: AnalyticsCatalog;
    analyticsViews: AnalyticsDashboardView[];
  };
}) {
  return (
    <PageShell
      eyebrow={data.activeWorkspace.name}
      title="Analytics"
      description="Trends across verdicts, policies, and latency for the selected environment."
    >
      <AnalyticsChartGrid
        catalog={data.analyticsCatalog}
        savedViews={data.analyticsViews}
      />
    </PageShell>
  );
}

export function RunDetailPageContent({
  data,
}: {
  data: DashboardShellData & {
    run: RunRow;
    events: RunEventRow[];
    traces: RunTraceRow[];
    liveSnapshot: RunDetailSnapshot;
  };
}) {
  return (
    <PageShell
      eyebrow={data.activeWorkspace.name}
      title="Run detail"
      description="The full decision timeline and traces for this guardrail check."
    >
      <RunDetailLiveView
        runId={data.run.id}
        workspaceSlug={data.activeWorkspace.slug}
        initialData={data.liveSnapshot}
      />
    </PageShell>
  );
}

export function KnowledgeSourcesPageContent({
  data,
}: {
  data: DashboardShellData & { knowledgeSources: KnowledgeSourceRow[] };
}) {
  return (
    <PageShell
      eyebrow={data.activeWorkspace.name}
      title="Knowledge"
      description="Documents the guardrail engine retrieves for context when evaluating a request."
      action={<KnowledgeSourceCreateDialog workspaceSlug={data.activeWorkspace.slug} />}
    >
      <SectionCard
        icon={IconBook2}
        eyebrow="Documents used for guardrail context"
        title="Knowledge sources"
        count={data.knowledgeSources.length}
      >
        {data.knowledgeSources.length === 0 ? (
          <EmptyState
            icon={<IconBook2 />}
            title="No knowledge sources yet"
            description="Upload a document or link a source so the guardrail engine can ground its decisions in your own content."
            action={<KnowledgeSourceCreateDialog workspaceSlug={data.activeWorkspace.slug} />}
          />
        ) : (
          <DataTable
            columns={knowledgeSourceColumns}
            rows={data.knowledgeSources}
            getRowKey={(source) => source.id}
            caption="Knowledge sources"
            empty="No knowledge sources yet."
          />
        )}
      </SectionCard>
    </PageShell>
  );
}

const knowledgeSourceColumns: DataTableColumn<KnowledgeSourceRow>[] = [
  { id: 'title', header: 'Source', cell: (row) => <span className="font-medium">{row.title}</span> },
  {
    id: 'kind',
    header: 'Kind',
    cell: (row) => row.kind,
    cellClassName: 'text-muted-foreground',
  },
  {
    id: 'location',
    header: 'Location',
    cellClassName: 'text-muted-foreground',
    cell: (row) =>
      row.downloadHref ? (
        <a
          className="inline-flex max-w-[18rem] items-center gap-1 font-data text-xs text-muted-foreground underline decoration-muted-foreground/40 underline-offset-4 transition-colors hover:text-foreground hover:decoration-foreground"
          href={row.downloadHref}
          target="_blank"
          rel="noopener noreferrer"
          title={row.location}
        >
          <span className="truncate">{row.location}</span>
          <IconExternalLink className="size-3 shrink-0" />
        </a>
      ) : (
        <span className="block max-w-[18rem] truncate font-data text-xs" title={row.location}>
          {row.location}
        </span>
      ),
  },
  {
    id: 'status',
    header: 'Status',
    cell: (row) => (
      <Badge variant="outline" className="font-data text-xs">
        {row.status}
      </Badge>
    ),
  },
  {
    id: 'lastIndexed',
    header: 'Last indexed',
    align: 'right',
    cell: (row) => row.lastIndexed,
    cellClassName: 'text-muted-foreground tabular-nums',
  },
];

export function TeamPageContent({
  data,
}: {
  data: DashboardShellData & {
    teamMembers: TeamMemberRow[];
    invites: TeamInviteRow[];
  };
}) {
  return (
    <PageShell
      eyebrow={data.organization.name}
      title="Team"
      description="Who can access this organization and the workspaces inside it."
      action={<InviteMemberDialog />}
    >
      <SectionCard
        icon={IconUsers}
        eyebrow="Organization members and workspace access"
        title="Members"
        count={data.teamMembers.length}
      >
        {data.teamMembers.length === 0 ? (
          <EmptyState
            icon={<IconUsers />}
            title="No team members yet"
            description="Invite a teammate to share guardrail configuration and monitoring across this organization."
            action={<InviteMemberDialog />}
          />
        ) : (
          <DataTable
            columns={teamMemberColumns}
            rows={data.teamMembers}
            getRowKey={(member) => member.id}
            caption="Organization members"
            empty="No team members yet."
          />
        )}
      </SectionCard>
      <SectionCard
        icon={IconUserCircle}
        eyebrow="Invited emails without an account yet — they join automatically on signup"
        title="Awaiting signup"
        count={data.invites.length}
      >
        <PendingInvitesTable invites={data.invites} />
      </SectionCard>
    </PageShell>
  );
}

const teamMemberColumns: DataTableColumn<TeamMemberRow>[] = [
  { id: 'name', header: 'Name', cell: (row) => <span className="font-medium">{row.name}</span> },
  {
    id: 'email',
    header: 'Email',
    cell: (row) => row.email,
    cellClassName: 'font-data text-xs text-muted-foreground',
  },
  {
    id: 'role',
    header: 'Role',
    cell: (row) => (
      <Badge variant="outline" className="font-data text-xs">
        {row.role}
      </Badge>
    ),
  },
  {
    id: 'access',
    header: 'Access',
    align: 'right',
    cell: (row) => row.access,
    cellClassName: 'text-muted-foreground',
  },
];

export function SettingsPageContent({ data }: { data: WorkspaceDashboardData }) {
  return (
    <PageShell
      eyebrow={data.activeWorkspace.name}
      title="Settings"
      description="Identity and runtime behavior for this workspace's guardrail engine."
      action={
        <Badge variant="outline" className="font-data gap-1.5 text-xs">
          <IconShieldCheck className="size-3.5" />
          Read-only preview
        </Badge>
      }
    >
      <p className="text-sm text-muted-foreground">
        These values mirror the live guardrail configuration. Editing settings from the dashboard is
        not available yet.
      </p>
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(320px,0.7fr)]">
        <SectionCard icon={IconSettings} eyebrow="Workspace identity" title="General">
          <div className="grid gap-5">
            <Field
              label="Workspace name"
              id="workspace-name"
              defaultValue={data.activeWorkspace.name}
              readOnly
              hint="Shown across the dashboard and in member invitations."
            />
            <Field
              label="Slug"
              id="workspace-slug"
              defaultValue={data.activeWorkspace.slug}
              mono
              readOnly
              hint="Used in URLs and API scoping. Keep it short and stable."
            />
            <div className="grid gap-2">
              <Label htmlFor="workspace-description">Description</Label>
              <Textarea
                id="workspace-description"
                defaultValue={data.activeWorkspace.description}
                placeholder="What this workspace guards"
                readOnly
                aria-readonly
                className="cursor-default text-muted-foreground"
              />
            </div>
          </div>
        </SectionCard>
        <SectionCard icon={IconShieldCheck} eyebrow="Runtime behavior" title="Guardrail config">
          <div className="grid gap-5">
            <Field
              label="Default action"
              id="default-action"
              defaultValue={data.settings.defaultAction}
              mono
              readOnly
              icon={IconGauge}
              hint="Applied when no policy matches a request."
            />
            <Field
              label="Escalation webhook"
              id="webhook"
              defaultValue={data.settings.escalationWebhookUrl ?? ''}
              placeholder="https://"
              mono
              readOnly
              icon={IconWebhook}
              hint="POSTed when a decision escalates for human review."
            />
            <Separator />
            <ToggleRow
              label="Telemetry"
              description="Store decision traces for dashboard review."
              id="telemetry-enabled"
              enabled={data.settings.telemetryEnabled}
              readOnly
            />
          </div>
        </SectionCard>
      </div>
    </PageShell>
  );
}

export function AccountPageContent({ data }: { data: DashboardShellData }) {
  return (
    <PageShell
      eyebrow="Your account"
      title="Account"
      description="Your profile and how TrustLoopGuard notifies you about guardrail events."
      action={
        <Badge variant="outline" className="font-data gap-1.5 text-xs">
          <IconUserCircle className="size-3.5" />
          Read-only preview
        </Badge>
      }
    >
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(320px,0.7fr)]">
        <SectionCard icon={IconId} eyebrow="Profile" title="Account details">
          <div className="grid gap-5">
            <Field label="Name" id="account-name" defaultValue={data.user.name} readOnly />
            <Field
              label="Email"
              id="account-email"
              defaultValue={data.user.email}
              mono
              readOnly
              hint="Used for sign-in and notification delivery."
            />
          </div>
        </SectionCard>
        <Card id="notifications" className="gap-0">
          <CardHeader className="border-b pb-5">
            <CardDescription className="flex items-center gap-1.5">
              <IconBell className="size-3.5" />
              Notifications
            </CardDescription>
            <CardTitle className="text-base">Preferences</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 pt-5">
            <ToggleRow
              label="Policy validation failures"
              description="Alert me when a policy fails to compile or validate."
              enabled
              readOnly
            />
            <ToggleRow
              label="Escalation delivery failures"
              description="Alert me when an escalation webhook does not deliver."
              enabled
              readOnly
            />
            <ToggleRow
              label="Weekly guardrail summary"
              description="A digest of decisions, blocks, and escalations."
              readOnly
            />
          </CardContent>
        </Card>
      </div>
    </PageShell>
  );
}

function PageShell({
  title,
  description,
  eyebrow,
  actionLabel,
  actionHref,
  actionIcon: ActionIcon,
  action,
  children,
}: {
  title: string;
  description: string;
  eyebrow?: string;
  actionLabel?: string;
  actionHref?: string;
  actionIcon?: Icon;
  action?: ReactNode;
  children: ReactNode;
}) {
  const resolvedActions =
    action ??
    (actionLabel && ActionIcon ? (
      actionHref ? (
        <Button asChild>
          <Link href={actionHref}>
            <ActionIcon />
            {actionLabel}
          </Link>
        </Button>
      ) : (
        <Button>
          <ActionIcon />
          {actionLabel}
        </Button>
      )
    ) : null);

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow={eyebrow}
        title={title}
        description={description}
        actions={resolvedActions}
      />
      {children}
    </div>
  );
}

/**
 * A card with a consistent labelled header (icon + eyebrow + title + optional
 * count badge) for the data sections on the management pages. Keeps the header
 * rhythm identical to the rest of the dashboard.
 */
function SectionCard({
  icon: SectionIcon,
  eyebrow,
  title,
  count,
  children,
}: {
  icon: Icon;
  eyebrow: string;
  title: string;
  count?: number;
  children: ReactNode;
}) {
  return (
    <Card className="gap-0 overflow-hidden">
      <CardHeader className="border-b pb-5">
        <CardDescription className="flex items-center gap-1.5">
          <SectionIcon className="size-3.5" />
          {eyebrow}
        </CardDescription>
        <CardTitle className="text-base">{title}</CardTitle>
        {count !== undefined ? (
          <CardAction>
            <Badge variant="secondary" className="font-data tabular-nums">
              {count}
            </Badge>
          </CardAction>
        ) : null}
      </CardHeader>
      <CardContent className="pt-5">{children}</CardContent>
    </Card>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border bg-muted/30 p-2.5">
      <div className="font-data text-lg font-semibold tabular-nums leading-none">{value}</div>
      <div className="mt-1.5 text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

function Field({
  label,
  id,
  defaultValue,
  hint,
  placeholder,
  mono = false,
  readOnly = false,
  icon: FieldIcon,
}: {
  label: string;
  id: string;
  defaultValue: string;
  hint?: string;
  placeholder?: string;
  mono?: boolean;
  readOnly?: boolean;
  icon?: Icon;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={id} className="flex items-center gap-1.5">
        {FieldIcon ? <FieldIcon className="size-3.5 text-muted-foreground" /> : null}
        {label}
      </Label>
      <Input
        id={id}
        defaultValue={defaultValue}
        placeholder={placeholder}
        readOnly={readOnly}
        aria-readonly={readOnly || undefined}
        className={cn(mono && 'font-data', readOnly && 'cursor-default text-muted-foreground')}
      />
      {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}

function ToggleRow({
  label,
  description,
  id,
  enabled = false,
  readOnly = false,
}: {
  label: string;
  description?: string;
  id?: string;
  enabled?: boolean;
  readOnly?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-md border bg-muted/20 p-3 transition-colors hover:bg-muted/40">
      <div className="grid gap-0.5">
        <Label htmlFor={id} className={readOnly ? undefined : 'cursor-pointer'}>
          {label}
        </Label>
        {description ? (
          <p className="text-sm text-muted-foreground">{description}</p>
        ) : null}
      </div>
      <Switch id={id} defaultChecked={enabled} disabled={readOnly} aria-readonly={readOnly || undefined} />
    </div>
  );
}
