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
import { InfoHint } from '@/components/ui/info-hint';
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
import { AgentEditDialog } from '@/components/workspace/AgentEditDialog';
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
      help={<InfoHint term="workspace" />}
      description="A workspace is a project space with its own rules, agents, and team. Jump into one here, or start a new workspace to keep a different product or client separate."
      actionLabel="New workspace"
      actionHref="/onboarding/workspace"
      actionIcon={IconPlus}
    >
      {data.workspaces.length === 0 ? (
        <EmptyState
          icon={<IconLayoutGrid />}
          title="No workspaces yet"
          description="Create your first workspace to set up rules and connect the AI apps you want to protect."
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
        <>
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
                    <Stat
                      label="Policies"
                      value={String(workspace.policyCount)}
                      help={<InfoHint label="What does “Policies” count mean?">Rules in this workspace that decide what to allow, rewrite, block, or hold for review.</InfoHint>}
                    />
                    <Stat
                      label="Agents"
                      value={String(workspace.agentCount)}
                      help={<InfoHint label="What does “Agents” count mean?">AI apps connected here that send their requests through the guardrail.</InfoHint>}
                    />
                    <Stat
                      label="Sources"
                      value={String(workspace.sourceCount)}
                      help={<InfoHint label="What does “Sources” count mean?">Trusted documents and links the guardrail can lean on when it makes a decision.</InfoHint>}
                    />
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
                      {isActive ? "You're here now" : 'Open this workspace'}
                      <IconArrowRight className="transition-transform group-hover:translate-x-0.5 motion-reduce:transition-none" />
                    </Link>
                  </Button>
                </CardFooter>
              </Card>
            );
          })}
          </div>
        </>
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
  {
    id: 'actions',
    header: <span className="sr-only">Actions</span>,
    align: 'right',
    cell: (row) => <AgentEditDialog agentId={row.id} agentName={row.name} />,
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
      help={<InfoHint term="agent" />}
      description="Agents are the AI apps and assistants you protect. Add one here to choose which rules guard it before its requests go out."
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
        eyebrow="Your AI apps"
        title="Agents"
        count={data.agents.length}
      >
        {data.agents.length === 0 ? (
          <EmptyState
            icon={<IconRobot />}
            title="No agents in this workspace yet"
            description="Add your first agent — the AI app you want to protect — to choose its rules and start checking its requests."
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
      help={<InfoHint term="run" />}
      description="A live stream of every request your agents sent through the guardrail, newest first. Open one to see exactly how it was decided."
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
      description="See how your guardrail is doing over time: how many requests were allowed, rewritten, blocked, or held for review, and how fast each check ran."
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
      help={<InfoHint term="trace" />}
      description="The step-by-step record of how this one request was checked and decided, from the rules that ran to the final outcome."
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
      help={<InfoHint term="knowledgeSource" />}
      description="The documents and links you've approved as trusted content. Add a source here so the guardrail can lean on your own material when it makes a decision."
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
            title="Add your first knowledge source"
            description="Knowledge sources are documents and links you approve as trusted content. Add one so the guardrail can check requests against your own material instead of guessing."
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
    header: (
      <span className="inline-flex items-center gap-1">
        Last indexed
        <InfoHint label="What does “Last indexed” mean?">
          When we last read this source and stored it so the guardrail can use it. Newly added
          sources show here once we&apos;ve finished reading them.
        </InfoHint>
      </span>
    ),
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
      help={<InfoHint term="role" />}
      description="Everyone who can access this organization, and what each person's role lets them do. Invite a teammate to share guardrail setup and monitoring."
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
            title="It's just you for now"
            description="Invite a teammate so they can help set up rules and keep an eye on what the guardrail is doing. You choose how much each person can do when you invite them."
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

/**
 * Plain-language meaning for each team role, shown next to the Role column so a
 * non-technical reader knows what a teammate can actually do. Falls back to a
 * general description for any role not listed here.
 */
const ROLE_MEANINGS: Record<string, string> = {
  owner: 'Can do everything, including billing, deleting the workspace, and managing every teammate.',
  admin: 'Can change rules, agents, and settings, and invite or remove teammates — but not delete the workspace.',
  editor: 'Can create and change rules and agents, but not manage members or settings.',
  viewer: 'Can look at everything and watch what the guardrail is doing, but can’t change anything.',
};

function roleMeaning(role: string): string {
  return (
    ROLE_MEANINGS[role.toLowerCase()] ??
    'What this teammate is allowed to do in the workspace.'
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
    header: (
      <span className="inline-flex items-center gap-1">
        Role
        <InfoHint term="role" />
      </span>
    ),
    cell: (row) => (
      <span className="inline-flex items-center gap-1">
        <Badge variant="outline" className="font-data text-xs">
          {row.role}
        </Badge>
        <InfoHint label={`What can a ${row.role} do?`}>{roleMeaning(row.role)}</InfoHint>
      </span>
    ),
  },
  {
    id: 'access',
    header: (
      <span className="inline-flex items-center gap-1">
        Access
        <InfoHint label="What does “Access” mean?">
          The workspaces this person can open and work in. Most teammates can reach just this one.
        </InfoHint>
      </span>
    ),
    align: 'right',
    cell: (row) => row.access,
    cellClassName: 'text-muted-foreground',
  },
];

/**
 * Turns the guardrail's default-action enum (allow / block / rewrite / escalate)
 * into a friendly phrase a non-technical reader can act on. Unknown values are
 * shown as-is so nothing is ever hidden.
 */
const DEFAULT_ACTION_LABELS: Record<string, string> = {
  allow: 'Let it through',
  block: 'Block it',
  rewrite: 'Clean it up, then allow',
  escalate: 'Hold it for a person to review',
};

function friendlyDefaultAction(value: string): string {
  return DEFAULT_ACTION_LABELS[value.toLowerCase()] ?? value;
}

export function SettingsPageContent({ data }: { data: WorkspaceDashboardData }) {
  return (
    <PageShell
      eyebrow={data.activeWorkspace.name}
      title="Settings"
      description="A live mirror of how this workspace's guardrail is configured and behaving right now."
      action={
        <Badge variant="outline" className="font-data gap-1.5 text-xs">
          <IconShieldCheck className="size-3.5" />
          Read-only preview
        </Badge>
      }
    >
      <p className="text-sm text-muted-foreground">
        Everything here is read straight from the guardrail that&apos;s running right now, so it
        always matches what&apos;s actually live. That&apos;s also why you can&apos;t edit it on this
        page — these settings are changed where the guardrail is configured, and this view just shows
        you the current setup.
      </p>
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(320px,0.7fr)]">
        <SectionCard icon={IconSettings} eyebrow="Workspace identity" title="General">
          <div className="grid gap-5">
            <Field
              label="Workspace name"
              id="workspace-name"
              defaultValue={data.activeWorkspace.name}
              readOnly
              hint="The friendly name you see across the dashboard and in invitations."
            />
            <Field
              label="Slug"
              id="workspace-slug"
              defaultValue={data.activeWorkspace.slug}
              mono
              readOnly
              help={
                <InfoHint label="What is a slug?">
                  A short, simple version of the name used in web links and behind the scenes. It
                  stays the same even if you rename the workspace.
                </InfoHint>
              }
              hint="A short tag for this workspace used in links. Best kept short and unchanged."
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
              defaultValue={friendlyDefaultAction(data.settings.defaultAction)}
              readOnly
              icon={IconGauge}
              help={
                <InfoHint label="What is the default action?">
                  What the guardrail does with a request when none of your rules specifically apply
                  to it — a safe fallback.
                </InfoHint>
              }
              hint="Used only when no rule matches a request."
            />
            <Field
              label="Escalation webhook"
              id="webhook"
              defaultValue={data.settings.escalationWebhookUrl ?? ''}
              placeholder="No address set"
              mono
              readOnly
              icon={IconWebhook}
              help={
                <InfoHint label="What is an escalation webhook?">
                  A web address we notify whenever a request is held for someone to review, so your
                  own tools can pick it up. Leave it blank if you don&apos;t use one.
                </InfoHint>
              }
              hint="Where we send a heads-up when a request needs a person to review it."
            />
            <Separator />
            <ToggleRow
              label="Telemetry"
              description="Keep a record of each decision so you can review it on the dashboard later. With this off, decisions still happen but aren't saved here."
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
      description="Your profile and sign-in details, plus how TrustLoopGuard keeps you posted about guardrail events."
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
            <Field
              label="Your name"
              id="account-name"
              defaultValue={data.user.name}
              readOnly
              hint="How you appear to teammates across the dashboard."
            />
            <Field
              label="Email address"
              id="account-email"
              defaultValue={data.user.email}
              mono
              readOnly
              hint="What you sign in with, and where we send your alerts and summaries."
            />
          </div>
        </SectionCard>
        <SectionCard
          id="notifications"
          icon={IconBell}
          eyebrow="Notifications"
          title="Preferences"
          badge={
            <Badge variant="outline" className="font-data text-xs">
              Preview
            </Badge>
          }
        >
          <div className="grid gap-3">
            <p className="text-sm text-muted-foreground">
              A preview of the alerts we&apos;re building. These toggles aren&apos;t adjustable
              yet — they&apos;re here so you can see what&apos;s coming.
            </p>
            <ToggleRow
              label="Policy validation failures"
              description="Alert me when a policy fails to compile or validate."
              enabled
              readOnly
              preview
            />
            <ToggleRow
              label="Escalation delivery failures"
              description="Alert me when an escalation webhook does not deliver."
              enabled
              readOnly
              preview
            />
            <ToggleRow
              label="Weekly guardrail summary"
              description="A digest of decisions, blocks, and escalations."
              readOnly
              preview
            />
          </div>
        </SectionCard>
      </div>
    </PageShell>
  );
}

function PageShell({
  title,
  description,
  help,
  eyebrow,
  actionLabel,
  actionHref,
  actionIcon: ActionIcon,
  action,
  children,
}: {
  title: string;
  description: string;
  help?: ReactNode;
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
        help={help}
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
  id,
  badge,
  children,
}: {
  icon: Icon;
  eyebrow: string;
  title: string;
  count?: number;
  id?: string;
  badge?: ReactNode;
  children: ReactNode;
}) {
  const action =
    badge ??
    (count !== undefined ? (
      <Badge variant="secondary" className="font-data tabular-nums">
        {count}
      </Badge>
    ) : null);
  return (
    <Card id={id} className="gap-0 overflow-hidden">
      <CardHeader className="border-b pb-5">
        <CardDescription className="flex items-center gap-1.5">
          <SectionIcon className="size-3.5" />
          {eyebrow}
        </CardDescription>
        <CardTitle className="text-base">{title}</CardTitle>
        {action ? <CardAction>{action}</CardAction> : null}
      </CardHeader>
      <CardContent className="pt-5">{children}</CardContent>
    </Card>
  );
}

function Stat({ label, value, help }: { label: string; value: string; help?: ReactNode }) {
  return (
    <div className="rounded-md border bg-muted/30 p-2.5">
      <div className="font-data text-lg font-semibold tabular-nums leading-none">{value}</div>
      <div className="mt-1.5 flex items-center gap-1 text-xs text-muted-foreground">
        {label}
        {help}
      </div>
    </div>
  );
}

function Field({
  label,
  id,
  defaultValue,
  hint,
  help,
  placeholder,
  mono = false,
  readOnly = false,
  icon: FieldIcon,
}: {
  label: string;
  id: string;
  defaultValue: string;
  hint?: string;
  help?: ReactNode;
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
        {help}
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
  preview = false,
}: {
  label: string;
  description?: string;
  id?: string;
  enabled?: boolean;
  readOnly?: boolean;
  preview?: boolean;
}) {
  // Derive a stable id from the label when none is given, so the Switch always has
  // an associated <Label> (this renders in a Server Component — no useId()).
  const controlId = id ?? `toggle-${label.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  const stateNote = preview
    ? 'Coming soon — not yet adjustable'
    : readOnly
      ? 'Read-only — shown for reference'
      : undefined;
  const ariaLabel = stateNote ? `${label} (${stateNote})` : undefined;
  return (
    <div className="flex items-center justify-between gap-4 rounded-md border bg-muted/20 p-3 transition-colors hover:bg-muted/40">
      <div className="grid gap-0.5">
        <Label htmlFor={controlId} className={readOnly ? undefined : 'cursor-pointer'}>
          {label}
        </Label>
        {description ? (
          <p className="text-sm text-muted-foreground">{description}</p>
        ) : null}
        {stateNote ? (
          <p className="text-xs font-medium text-muted-foreground/80">{stateNote}</p>
        ) : null}
      </div>
      <Switch
        id={controlId}
        defaultChecked={enabled}
        disabled={readOnly}
        aria-label={ariaLabel}
      />
    </div>
  );
}
