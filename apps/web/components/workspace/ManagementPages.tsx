import Link from 'next/link';
import {
  IconCheck,
  IconPlus,
  IconRobot,
  IconUsers,
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
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { InviteMemberDialog } from '@/components/workspace/InviteMemberDialog';
import { KnowledgeSourceCreateDialog } from '@/components/workspace/KnowledgeSourceCreateDialog';
import { PendingInvitesTable } from '@/components/workspace/PendingInvitesTable';
import { PolicyCreateDialog } from '@/components/workspace/PolicyCreateDialog';
import type {
  AgentRow,
  DashboardShellData,
  KnowledgeSourceRow,
  PolicyRow,
  TeamInviteRow,
  TeamMemberRow,
  WorkspaceDashboardData,
} from '@/lib/server/dashboard-data';

export function WorkspacesPageContent({ data }: { data: DashboardShellData }) {
  return (
    <PageShell
      description={data.organization.name}
      title="Workspaces"
      actionLabel="New workspace"
      actionHref="/onboarding/workspace"
      actionIcon={IconPlus}
    >
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {data.workspaces.map((workspace) => (
          <Card key={workspace.id}>
            <CardHeader>
              <CardDescription>{workspace.role}</CardDescription>
              <CardTitle>{workspace.name}</CardTitle>
              <CardAction>
                {workspace.id === data.activeWorkspace.id ? (
                  <Badge variant="outline" className="rounded-sm">
                    active
                  </Badge>
                ) : null}
              </CardAction>
            </CardHeader>
            <CardContent className="grid gap-4">
              <p className="text-sm text-muted-foreground">{workspace.description}</p>
              <div className="grid grid-cols-3 gap-2 text-sm">
                <Stat label="Policies" value={String(workspace.policyCount)} />
                <Stat label="Agents" value={String(workspace.agentCount)} />
                <Stat label="Sources" value={String(workspace.sourceCount)} />
              </div>
              <Button asChild variant="outline" size="sm">
                <Link href={`/?workspace=${workspace.slug}`}>Open workspace</Link>
              </Button>
            </CardContent>
          </Card>
        ))}
      </div>
    </PageShell>
  );
}

const agentColumns: DataTableColumn<AgentRow>[] = [
  { id: 'name', header: 'Agent', cell: (row) => row.name },
  {
    id: 'scope',
    header: 'Scope',
    cell: (row) => row.scope,
    cellClassName: 'text-muted-foreground',
  },
  { id: 'policies', header: 'Policies', cell: (row) => row.policies },
  {
    id: 'status',
    header: 'Status',
    cell: (row) => (
      <Badge variant="outline" className="rounded-sm">
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
    <PageShell title="Agents" description={data.activeWorkspace.name} actionLabel="New agent" actionIcon={IconRobot}>
      <Card>
        <CardHeader>
          <CardDescription>Workspace-owned agent profiles</CardDescription>
          <CardTitle>Guardrail agents</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={agentColumns}
            rows={data.agents}
            getRowKey={(agent) => agent.id}
            empty="No agents in this workspace yet."
          />
        </CardContent>
      </Card>
    </PageShell>
  );
}

export function PoliciesPageContent({
  data,
}: {
  data: DashboardShellData & { agents: AgentRow[]; policies: PolicyRow[] };
}) {
  return (
    <PageShell
      title="Policies"
      description={data.activeWorkspace.name}
      action={
        <PolicyCreateDialog agents={data.agents} workspaceSlug={data.activeWorkspace.slug}>
          <IconPlus />
          New policy
        </PolicyCreateDialog>
      }
    >
      <Card>
        <CardHeader>
          <CardDescription>Workspace-authored guardrails</CardDescription>
          <CardTitle>Policies</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={policyColumns}
            rows={data.policies}
            getRowKey={(policy) => policy.id}
            empty="No policies authored yet."
          />
        </CardContent>
      </Card>
    </PageShell>
  );
}

const policyColumns: DataTableColumn<PolicyRow>[] = [
  {
    id: 'id',
    header: 'Policy',
    cell: (row) => row.id,
    cellClassName: 'font-mono text-xs',
  },
  {
    id: 'description',
    header: 'Description',
    cell: (row) => row.description,
    cellClassName: 'text-muted-foreground',
  },
  { id: 'agent', header: 'Agent', cell: (row) => row.agent },
  {
    id: 'severity',
    header: 'Severity',
    cell: (row) => (
      <Badge variant="outline" className="rounded-sm">
        {row.severity}
      </Badge>
    ),
  },
  { id: 'action', header: 'Action', cell: (row) => row.action },
  { id: 'enabled', header: 'Enabled', cell: (row) => (row.enabled ? 'Yes' : 'No') },
];

export function KnowledgeSourcesPageContent({
  data,
}: {
  data: DashboardShellData & { knowledgeSources: KnowledgeSourceRow[] };
}) {
  return (
    <PageShell
      title="Knowledge"
      description={data.activeWorkspace.name}
      action={<KnowledgeSourceCreateDialog workspaceSlug={data.activeWorkspace.slug} />}
    >
      <Card>
        <CardHeader>
          <CardDescription>Documents used for guardrail context</CardDescription>
          <CardTitle>Knowledge sources</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={knowledgeSourceColumns}
            rows={data.knowledgeSources}
            getRowKey={(source) => source.id}
            empty="No knowledge sources yet."
          />
        </CardContent>
      </Card>
    </PageShell>
  );
}

const knowledgeSourceColumns: DataTableColumn<KnowledgeSourceRow>[] = [
  { id: 'title', header: 'Source', cell: (row) => row.title },
  { id: 'kind', header: 'Kind', cell: (row) => row.kind },
  {
    id: 'location',
    header: 'Location',
    cellClassName: 'text-muted-foreground',
    cell: (row) =>
      row.downloadHref ? (
        <a className="underline-offset-4 hover:underline" href={row.downloadHref}>
          {row.location}
        </a>
      ) : (
        row.location
      ),
  },
  {
    id: 'status',
    header: 'Status',
    cell: (row) => (
      <Badge variant="outline" className="rounded-sm">
        {row.status}
      </Badge>
    ),
  },
  {
    id: 'lastIndexed',
    header: 'Last indexed',
    cell: (row) => row.lastIndexed,
    cellClassName: 'text-muted-foreground',
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
      title="Team"
      description={data.organization.name}
      action={<InviteMemberDialog />}
    >
      <Card>
        <CardHeader>
          <CardDescription>Organization members and workspace access</CardDescription>
          <CardTitle>Members</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={teamMemberColumns}
            rows={data.teamMembers}
            getRowKey={(member) => member.id}
            empty="No team members yet."
          />
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardDescription>
            Emails we&apos;ve invited that don&apos;t have an account yet —
            they&apos;ll join automatically when they sign up.
          </CardDescription>
          <CardTitle>Awaiting signup</CardTitle>
        </CardHeader>
        <CardContent>
          <PendingInvitesTable invites={data.invites} />
        </CardContent>
      </Card>
    </PageShell>
  );
}

const teamMemberColumns: DataTableColumn<TeamMemberRow>[] = [
  { id: 'name', header: 'Name', cell: (row) => row.name },
  {
    id: 'email',
    header: 'Email',
    cell: (row) => row.email,
    cellClassName: 'text-muted-foreground',
  },
  {
    id: 'role',
    header: 'Role',
    cell: (row) => (
      <Badge variant="outline" className="rounded-sm">
        {row.role}
      </Badge>
    ),
  },
  { id: 'access', header: 'Access', cell: (row) => row.access },
];

export function SettingsPageContent({ data }: { data: WorkspaceDashboardData }) {
  return (
    <PageShell title="Settings" description={data.activeWorkspace.name} actionLabel="Save changes" actionIcon={IconCheck}>
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <Card>
          <CardHeader>
            <CardDescription>Workspace identity</CardDescription>
            <CardTitle>General</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            <Field label="Workspace name" id="workspace-name" defaultValue={data.activeWorkspace.name} />
            <Field label="Slug" id="workspace-slug" defaultValue={data.activeWorkspace.slug} />
            <div className="grid gap-2">
              <Label htmlFor="workspace-description">Description</Label>
              <Textarea id="workspace-description" defaultValue={data.activeWorkspace.description} />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardDescription>Runtime behavior</CardDescription>
            <CardTitle>Guardrail config</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            <Field label="Default action" id="default-action" defaultValue={data.settings.defaultAction} />
            <Field
              label="Escalation webhook"
              id="webhook"
              defaultValue={data.settings.escalationWebhookUrl ?? ''}
            />
            <div className="flex items-center justify-between gap-3 border p-3">
              <div>
                <Label htmlFor="telemetry-enabled">Telemetry</Label>
                <p className="text-sm text-muted-foreground">Store decision traces for dashboard review.</p>
              </div>
              <Switch id="telemetry-enabled" defaultChecked={data.settings.telemetryEnabled} />
            </div>
          </CardContent>
        </Card>
      </div>
    </PageShell>
  );
}

export function AccountPageContent({ data }: { data: DashboardShellData }) {
  return (
    <PageShell title="Account" description="User profile and notification preferences">
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <Card>
          <CardHeader>
            <CardDescription>Profile</CardDescription>
            <CardTitle>Account details</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            <Field label="Name" id="account-name" defaultValue={data.user.name} />
            <Field label="Email" id="account-email" defaultValue={data.user.email} />
          </CardContent>
        </Card>
        <Card id="notifications">
          <CardHeader>
            <CardDescription>Notifications</CardDescription>
            <CardTitle>Preferences</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            <ToggleRow label="Policy validation failures" enabled />
            <ToggleRow label="Escalation delivery failures" enabled />
            <ToggleRow label="Weekly guardrail summary" />
          </CardContent>
        </Card>
      </div>
    </PageShell>
  );
}

function PageShell({
  title,
  description,
  actionLabel,
  actionHref,
  actionIcon: ActionIcon,
  action,
  children,
}: {
  title: string;
  description: string;
  actionLabel?: string;
  actionHref?: string;
  actionIcon?: Icon;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">{description}</p>
          <h2 className="text-2xl font-semibold">{title}</h2>
        </div>
        {action ??
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
        ) : null)}
      </div>
      {children}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="border p-3">
      <div className="text-lg font-semibold tabular-nums">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

function Field({ label, id, defaultValue }: { label: string; id: string; defaultValue: string }) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} defaultValue={defaultValue} />
    </div>
  );
}

function ToggleRow({ label, enabled = false }: { label: string; enabled?: boolean }) {
  return (
    <div className="flex items-center justify-between gap-3 border p-3">
      <Label>{label}</Label>
      <Switch defaultChecked={enabled} />
    </div>
  );
}
