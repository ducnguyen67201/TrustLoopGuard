import 'server-only';

import { redirect } from 'next/navigation';

import { auth } from '@/auth';
import {
  normalizeWorkspaceSlug,
  rustApiForUser,
  rustApiForWorkspace,
  workspaceIdFromSlug,
} from './tl-client';

export interface DashboardShellData {
  user: {
    name: string;
    email: string;
    avatar: string;
  };
  organization: {
    id: string;
    name: string;
    slug: string;
  };
  activeWorkspace: WorkspaceSummary;
  workspaces: WorkspaceSummary[];
}

export interface WorkspaceSummary {
  id: string;
  name: string;
  slug: string;
  description: string;
  policyCount: number;
  enabledPolicies: number;
  agentCount: number;
  sourceCount: number;
  apiKeyCount: number;
  role: string;
}

export interface WorkspaceDashboardData extends DashboardShellData {
  metrics: Array<{
    label: string;
    value: string;
    delta: string;
    detail: string;
  }>;
  recentDecisions: Array<{
    id: string;
    agent: string;
    verdict: string;
    policy: string;
    latency: string;
    time: string;
  }>;
  settings: {
    defaultAction: string;
    escalationWebhookUrl: string | null;
    telemetryEnabled: boolean;
    retentionDays: string;
  };
}

export type AgentRow = {
  id: string;
  name: string;
  scope: string;
  policies: number;
  status: string;
};

export type KnowledgeSourceRow = {
  id: string;
  title: string;
  kind: string;
  location: string;
  downloadHref: string | null;
  status: string;
  lastIndexed: string;
};

export type ApiKeyRow = {
  id: string;
  name: string;
  prefix: string;
  status: string;
  lastUsed: string;
  createdBy: string;
};

export type TeamMemberRow = {
  id: string;
  name: string;
  email: string;
  role: string;
  access: string;
};

export type TeamInviteRow = {
  id: string;
  email: string;
  role: string;
  status: string;
  invitedAt: string;
  expiresAt: string;
  acceptPath: string;
};

export type PolicyRow = {
  id: string;
  description: string;
  severity: string;
  action: string;
  enabled: boolean;
  agent: string;
};

type CurrentUser = {
  id: string;
  name: string;
  email: string;
  image: string;
};

type RuntimeDecisionPayload = {
  trace_id?: string;
  verdict?: string;
  reason?: string;
  triggered_policies?: Array<{ id?: string; severity?: string; reason?: string }>;
  safe_output?: string | null;
  latency_ms?: number;
  agent_id?: string;
};

type AgentProfileWire = {
  agent_id: string;
  display_name?: string;
  scope?: {
    in_scope?: string[];
    out_of_scope?: string[];
  };
  system_prompt?: string;
};

type AgentListWire = {
  agents: AgentProfileWire[];
};

type PolicySummaryWire = {
  id: string;
  description?: string | null;
  severity: string;
  action?: string;
  enabled: boolean;
  owner_agent_id?: string | null;
};

type PolicyListWire = {
  policies: PolicySummaryWire[];
};

type TraceSummaryWire = {
  trace_id: string;
  domain: string;
  decision: string;
  elapsed_ms: number;
  payload: RuntimeDecisionPayload;
  created_at: string;
};

type TraceListWire = {
  traces: TraceSummaryWire[];
};

type ApiKeyWire = {
  id: string;
  name: string;
  prefix: string;
  status: string;
  created_at: string;
  last_used_at: string | null;
  created_by: string | null;
};

type ApiKeyListWire = {
  api_keys: ApiKeyWire[];
};

type WorkspaceSettingsWire = {
  default_action: string;
  escalation_webhook_url: string | null;
  telemetry_enabled: boolean;
  retention_days: string;
  config: Record<string, unknown>;
  updated_at: string | null;
};

type KnowledgeSourceWire = {
  id: string;
  title: string;
  kind: string;
  location: string | null;
  status: string;
  metadata: Record<string, unknown>;
  last_indexed_at: string | null;
};

type KnowledgeSourceListWire = {
  knowledge_sources: KnowledgeSourceWire[];
};

export async function getDashboardShell(workspaceSlug?: string | null): Promise<DashboardShellData> {
  const user = await getCurrentUser();
  return buildDashboardShell(user, workspaceSlug);
}

export async function getOptionalDashboardShell(
  workspaceSlug?: string | null,
): Promise<DashboardShellData | null> {
  const user = await findCurrentUser();
  if (!user) return null;
  return buildDashboardShell(user, workspaceSlug);
}

export async function getWorkspaceDashboard(
  workspaceSlug?: string | null,
): Promise<WorkspaceDashboardData> {
  const shell = await getDashboardShell(workspaceSlug);
  const workspaceId = shell.activeWorkspace.id;
  const recentDecisions = (
    await rustApiForWorkspace<TraceListWire>(workspaceId, '/v1/traces?limit=8')
  ).traces;
  const blocked = recentDecisions.filter((decision) => decision.decision === 'block').length;
  const escalated = recentDecisions.filter((decision) => decision.decision === 'escalate').length;

  return {
    ...shell,
    metrics: [
      {
        label: 'Decisions',
        value: String(recentDecisions.length),
        delta: 'live',
        detail: 'Recent workspace traces',
      },
      {
        label: 'Blocked',
        value: String(blocked),
        delta: `${blocked}/${recentDecisions.length}`,
        detail: 'Policy blocks in current sample',
      },
      {
        label: 'Escalated',
        value: String(escalated),
        delta: `${escalated}/${recentDecisions.length}`,
        detail: 'Sent to human review',
      },
      {
        label: 'p95 latency',
        value: p95Latency(recentDecisions.map((decision) => Number(decision.elapsed_ms))),
        delta: 'runtime',
        detail: 'Runtime guardrail checks',
      },
    ],
    recentDecisions: recentDecisions.map((decision) => ({
      id: String(decision.trace_id),
      agent: readTraceAgent(decision.payload),
      verdict: decision.decision,
      policy: readTracePolicy(decision.payload),
      latency: `${decision.elapsed_ms}ms`,
      time: relativeTime(new Date(decision.created_at)),
    })),
    settings: settingsFromWire(await getWorkspaceSettings(workspaceId)),
  };
}

export async function getWorkspacesPageData(workspaceSlug?: string | null) {
  return getDashboardShell(workspaceSlug);
}

export async function getOnboardingUser() {
  return getCurrentUser();
}

export async function getAgentsPageData(
  workspaceSlug?: string | null,
): Promise<DashboardShellData & { agents: AgentRow[] }> {
  const shell = await getDashboardShell(workspaceSlug);
  const policies = await listPolicyRows(shell.activeWorkspace.id);
  return {
    ...shell,
    agents: await listAgentRows(shell.activeWorkspace.id, policies),
  };
}

export async function getKnowledgePageData(
  workspaceSlug?: string | null,
): Promise<DashboardShellData & { knowledgeSources: KnowledgeSourceRow[] }> {
  const shell = await getDashboardShell(workspaceSlug);
  const rows = (
    await rustApiForWorkspace<KnowledgeSourceListWire>(
      shell.activeWorkspace.id,
      '/v1/knowledge-sources',
    )
  ).knowledge_sources;

  return {
    ...shell,
    knowledgeSources: rows.map((source) => ({
      id: source.id,
      title: source.title,
      kind: titleize(source.kind),
      location: source.location ?? 'Not set',
      downloadHref:
        source.kind === 'file'
          ? `/api/knowledge-sources/${source.id}/file?workspace=${shell.activeWorkspace.slug}`
          : null,
      status: titleize(source.status),
      lastIndexed: source.last_indexed_at ? relativeTime(new Date(source.last_indexed_at)) : 'Not indexed',
    })),
  };
}

export async function getApiKeysPageData(
  workspaceSlug?: string | null,
): Promise<DashboardShellData & { apiKeys: ApiKeyRow[] }> {
  const shell = await getDashboardShell(workspaceSlug);
  const rows = (
    await rustApiForWorkspace<ApiKeyListWire>(shell.activeWorkspace.id, '/v1/api-keys')
  ).api_keys;
  return {
    ...shell,
    apiKeys: rows.map((key) => ({
      id: key.id,
      name: key.name,
      prefix: key.prefix,
      status: titleize(key.status),
      lastUsed: key.last_used_at ? relativeTime(new Date(key.last_used_at)) : 'Never',
      createdBy: key.created_by ?? 'System',
    })),
  };
}

interface RustMember {
  user_id: string;
  username: string;
  role: string;
  joined_at: string;
}

interface RustInvite {
  id: string;
  email: string;
  role: string;
  status: string;
  created_at: string;
  expires_at: string;
}

export async function getTeamPageData(
  workspaceSlug?: string | null,
): Promise<
  DashboardShellData & { teamMembers: TeamMemberRow[]; invites: TeamInviteRow[] }
> {
  const shell = await getDashboardShell(workspaceSlug);
  const workspaceId = workspaceIdFromSlug(workspaceSlug);

  const [members, invites] = await Promise.all([
    rustApiForWorkspace<{ members: RustMember[] }>(workspaceId, '/v1/team/members').catch(
      () => ({ members: [] as RustMember[] }),
    ),
    rustApiForWorkspace<{ invites: RustInvite[] }>(workspaceId, '/v1/team/invites').catch(
      () => ({ invites: [] as RustInvite[] }),
    ),
  ]);

  const accessList = shell.workspaces.map((workspace) => workspace.name).join(', ');

  const teamMembers: TeamMemberRow[] =
    members.members.length > 0
      ? members.members.map((m) => ({
          id: m.user_id,
          name: m.username,
          email: m.username,
          role: titleize(m.role),
          access: shell.activeWorkspace.name,
        }))
      : [
          {
            id: shell.user.email,
            name: shell.user.name,
            email: shell.user.email,
            role: 'Owner',
            access: accessList,
          },
        ];

  const inviteRows: TeamInviteRow[] = invites.invites.map((i) => ({
    id: i.id,
    email: i.email,
    role: titleize(i.role),
    status: titleize(i.status),
    invitedAt: relativeTime(new Date(i.created_at)),
    expiresAt: relativeTime(new Date(i.expires_at)),
    acceptPath: `/invite/accept?token=${encodeURIComponent(i.id)}`,
  }));

  return {
    ...shell,
    teamMembers,
    invites: inviteRows,
  };
}

export async function getSettingsPageData(workspaceSlug?: string | null) {
  const shell = await getDashboardShell(workspaceSlug);
  return {
    ...shell,
    metrics: [],
    recentDecisions: [],
    settings: settingsFromWire(await getWorkspaceSettings(shell.activeWorkspace.id)),
  };
}

export async function getPoliciesPageData(
  workspaceSlug?: string | null,
): Promise<DashboardShellData & { agents: AgentRow[]; policies: PolicyRow[] }> {
  const shell = await getDashboardShell(workspaceSlug);
  const policies = await listPolicyRows(shell.activeWorkspace.id);
  return {
    ...shell,
    agents: await listAgentRows(shell.activeWorkspace.id, policies),
    policies,
  };
}

async function getCurrentUser(): Promise<CurrentUser> {
  const user = await findCurrentUser();
  if (!user) {
    redirect('/signin');
  }
  return user;
}

async function findCurrentUser(): Promise<CurrentUser | null> {
  const session = await auth();
  const sessionUser = session?.user;
  if (!sessionUser?.id) return null;

  const username =
    (sessionUser as { username?: string }).username?.trim() ||
    sessionUser.name?.trim() ||
    sessionUser.email?.trim() ||
    'User';

  return {
    id: sessionUser.id,
    name: username,
    email: sessionUser.email?.trim() || username,
    image: sessionUser.image ?? '',
  };
}

interface MyWorkspaceWire {
  id: string;
  slug: string;
  name: string;
  role: string;
  organization_id: string;
}

interface MyWorkspacesWire {
  workspaces: MyWorkspaceWire[];
}

/// Membership lookup with auto-bind for pending invites. Returns an
/// empty list rather than throwing when Rust is unreachable — callers
/// (the dashboard shell, the welcome page) decide what to do with
/// zero memberships.
export async function getMyWorkspaces(user: CurrentUser): Promise<MyWorkspaceWire[]> {
  try {
    const data = await rustApiForUser<MyWorkspacesWire>(
      { id: user.id, email: user.email },
      '/v1/team/my-workspaces',
    );
    return data.workspaces;
  } catch {
    return [];
  }
}

async function buildDashboardShell(
  user: CurrentUser,
  workspaceSlug?: string | null,
): Promise<DashboardShellData> {
  const memberships = await getMyWorkspaces(user);
  if (memberships.length === 0) {
    // No workspace = nothing to render. Bounce to the welcome page so
    // the user sees what to do next (wait for an invite). Pages that
    // render in user-state-agnostic shells (e.g. /welcome itself) call
    // getOptionalDashboardShell instead and avoid this branch.
    redirect('/welcome');
  }

  const requested = workspaceSlug?.trim();
  const active =
    (requested !== undefined && requested !== ''
      ? memberships.find((m) => m.slug === requested)
      : undefined) ?? memberships[0]!;

  const summary = await buildWorkspaceSummary(active.slug, active);
  const all = await Promise.all(
    memberships.map((m) =>
      m.slug === active.slug ? Promise.resolve(summary) : buildWorkspaceSummary(m.slug, m),
    ),
  );

  return {
    user: {
      name: user.name,
      email: user.email,
      avatar: user.image,
    },
    organization: {
      id: active.organization_id,
      name: summary.name,
      slug: summary.slug,
    },
    activeWorkspace: summary,
    workspaces: all,
  };
}

async function buildWorkspaceSummary(
  workspaceSlug?: string | null,
  membership?: MyWorkspaceWire,
): Promise<WorkspaceSummary> {
  const slug = membership?.slug ?? normalizeWorkspaceSlug(workspaceSlug);
  const id = membership?.id ?? `ws_${slug.replace(/-/g, '_')}`;
  const [policyList, agentList, knowledgeList] = await Promise.all([
    rustApiForWorkspace<PolicyListWire>(id, '/v1/policies'),
    rustApiForWorkspace<AgentListWire>(id, '/v1/agents'),
    rustApiForWorkspace<KnowledgeSourceListWire>(id, '/v1/knowledge-sources'),
  ]);
  const policyCount = policyList.policies.length;

  return {
    id,
    name: membership?.name ?? titleize(slug),
    slug,
    description: 'Workspace-managed guardrail configuration.',
    policyCount,
    enabledPolicies: policyList.policies.filter((policy) => policy.enabled).length,
    agentCount: agentList.agents.length,
    sourceCount: knowledgeList.knowledge_sources.length,
    apiKeyCount: 0,
    role: membership?.role ?? 'Owner',
  };
}

async function listPolicyRows(workspaceId: string): Promise<PolicyRow[]> {
  const [policyList, agentList] = await Promise.all([
    rustApiForWorkspace<PolicyListWire>(workspaceId, '/v1/policies'),
    rustApiForWorkspace<AgentListWire>(workspaceId, '/v1/agents'),
  ]);
  const agentsById = new Map(agentList.agents.map((agent) => [agent.agent_id, agent]));

  return policyList.policies.map((policy) => ({
    id: policy.id,
    description: policy.description?.trim() || 'Runtime policy',
    severity: policy.severity ?? 'medium',
    action: policy.action ?? 'block',
    enabled: policy.enabled,
    agent: policy.owner_agent_id
      ? agentName(agentsById.get(policy.owner_agent_id) ?? null, policy.owner_agent_id)
      : 'Global',
  }));
}

async function listAgentRows(workspaceId: string, policies: PolicyRow[]): Promise<AgentRow[]> {
  const rows = (await rustApiForWorkspace<AgentListWire>(workspaceId, '/v1/agents')).agents;

  return rows.map((agent) => ({
    id: agent.agent_id,
    name: agentName(agent, agent.agent_id),
    scope: agentScope(agent),
    policies: policies.filter((policy) => policy.agent === agentName(agent, agent.agent_id)).length,
    status: 'Ready',
  }));
}

async function getWorkspaceSettings(workspaceId: string): Promise<WorkspaceSettingsWire> {
  return rustApiForWorkspace<WorkspaceSettingsWire>(workspaceId, '/v1/settings');
}

function settingsFromWire(settings: WorkspaceSettingsWire): WorkspaceDashboardData['settings'] {
  return {
    defaultAction: settings.default_action,
    escalationWebhookUrl: settings.escalation_webhook_url,
    telemetryEnabled: settings.telemetry_enabled,
    retentionDays: settings.retention_days,
  };
}

function agentName(profile: AgentProfileWire | null | undefined, fallback: string): string {
  return profile?.display_name?.trim() || profile?.agent_id?.trim() || fallback;
}

function agentScope(profile: AgentProfileWire): string {
  const inScope = profile.scope?.in_scope?.filter(Boolean) ?? [];
  if (inScope.length > 0) return inScope.join(', ');
  return 'Runtime agent';
}

function readTraceAgent(payload: RuntimeDecisionPayload): string {
  return payload.agent_id?.trim() || 'Runtime agent';
}

function readTracePolicy(payload: RuntimeDecisionPayload): string {
  const [policy] = payload.triggered_policies ?? [];
  return policy?.id?.trim() || 'baseline';
}

function p95Latency(values: number[]): string {
  if (values.length === 0) return '0ms';
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * 0.95) - 1);
  return `${sorted[index]}ms`;
}

function relativeTime(date: Date): string {
  const diffMs = Date.now() - date.getTime();
  const minutes = Math.max(1, Math.round(diffMs / 60000));
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}

function titleize(value: string): string {
  return value
    .split(/[-_]/)
    .filter(Boolean)
    .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join(' ');
}
