import Link from 'next/link';
import {
  IconCheck,
  IconKey,
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
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { KnowledgeSourceCreateDialog } from '@/components/workspace/KnowledgeSourceCreateDialog';
import { PolicyCreateDialog } from '@/components/workspace/PolicyCreateDialog';
import type {
  AgentRow,
  ApiKeyRow,
  DashboardShellData,
  KnowledgeSourceRow,
  PolicyRow,
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
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Agent</TableHead>
                <TableHead>Scope</TableHead>
                <TableHead>Policies</TableHead>
                <TableHead>Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.agents.map((agent) => (
                <TableRow key={agent.id}>
                  <TableCell>{agent.name}</TableCell>
                  <TableCell className="text-muted-foreground">{agent.scope}</TableCell>
                  <TableCell>{agent.policies}</TableCell>
                  <TableCell>
                    <Badge variant="outline" className="rounded-sm">
                      {agent.status}
                    </Badge>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
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
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Policy</TableHead>
                <TableHead>Description</TableHead>
                <TableHead>Agent</TableHead>
                <TableHead>Severity</TableHead>
                <TableHead>Action</TableHead>
                <TableHead>Enabled</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.policies.map((policy) => (
                <TableRow key={policy.id}>
                  <TableCell className="font-mono text-xs">{policy.id}</TableCell>
                  <TableCell className="text-muted-foreground">{policy.description}</TableCell>
                  <TableCell>{policy.agent}</TableCell>
                  <TableCell>
                    <Badge variant="outline" className="rounded-sm">
                      {policy.severity}
                    </Badge>
                  </TableCell>
                  <TableCell>{policy.action}</TableCell>
                  <TableCell>{policy.enabled ? 'Yes' : 'No'}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
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
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Source</TableHead>
                <TableHead>Kind</TableHead>
                <TableHead>Location</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Last indexed</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.knowledgeSources.map((source) => (
                <TableRow key={source.id}>
                  <TableCell>{source.title}</TableCell>
                  <TableCell>{source.kind}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {source.downloadHref ? (
                      <a className="underline-offset-4 hover:underline" href={source.downloadHref}>
                        {source.location}
                      </a>
                    ) : (
                      source.location
                    )}
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="rounded-sm">
                      {source.status}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground">{source.lastIndexed}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </PageShell>
  );
}

export function ApiKeysPageContent({
  data,
}: {
  data: DashboardShellData & { apiKeys: ApiKeyRow[] };
}) {
  return (
    <PageShell title="API Keys" description={data.activeWorkspace.name} actionLabel="Create key" actionIcon={IconKey}>
      <Card>
        <CardHeader>
          <CardDescription>Workspace-scoped runtime credentials</CardDescription>
          <CardTitle>API keys</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Prefix</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Last used</TableHead>
                <TableHead>Created by</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.apiKeys.map((apiKey) => (
                <TableRow key={apiKey.id}>
                  <TableCell>{apiKey.name}</TableCell>
                  <TableCell className="font-mono text-xs">{apiKey.prefix}</TableCell>
                  <TableCell>
                    <Badge variant="outline" className="rounded-sm">
                      {apiKey.status}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground">{apiKey.lastUsed}</TableCell>
                  <TableCell>{apiKey.createdBy}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </PageShell>
  );
}

export function TeamPageContent({
  data,
}: {
  data: DashboardShellData & { teamMembers: TeamMemberRow[] };
}) {
  return (
    <PageShell title="Team" description={data.organization.name} actionLabel="Invite member" actionIcon={IconUsers}>
      <Card>
        <CardHeader>
          <CardDescription>Organization members and workspace access</CardDescription>
          <CardTitle>Members</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Email</TableHead>
                <TableHead>Role</TableHead>
                <TableHead>Access</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.teamMembers.map((member) => (
                <TableRow key={member.id}>
                  <TableCell>{member.name}</TableCell>
                  <TableCell className="text-muted-foreground">{member.email}</TableCell>
                  <TableCell>
                    <Badge variant="outline" className="rounded-sm">
                      {member.role}
                    </Badge>
                  </TableCell>
                  <TableCell>{member.access}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </PageShell>
  );
}

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
