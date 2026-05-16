import 'server-only';

import { and, count, desc, eq, isNull } from 'drizzle-orm';
import { redirect } from 'next/navigation';

import { auth } from '@/auth';
import { getDb } from '@/lib/db/client';
import { users } from '@/lib/db/schema/auth';
import {
  knowledgeSources,
  organizationMembers,
  organizations,
  workspaceApiKeys,
  workspaceMembers,
  runtimeAgents,
  runtimePolicies,
  runtimeTraces,
  type RuntimeAgentProfile,
  type RuntimeDecisionPayload,
  type RuntimePolicyDocument,
  workspaces,
  workspaceSettings,
} from '@/lib/db/schema/workspace';

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

export type PolicyRow = {
  id: string;
  description: string;
  severity: string;
  action: string;
  enabled: boolean;
  agent: string;
};

export async function getDashboardShell(workspaceSlug?: string | null): Promise<DashboardShellData> {
  const user = await getCurrentUser();
  return getDashboardShellForUser(user, workspaceSlug);
}

export async function getOptionalDashboardShell(
  workspaceSlug?: string | null,
): Promise<DashboardShellData | null> {
  const user = await findCurrentUser();
  if (!user) return null;

  const workspaceRows = await listWorkspaceSummaries(user.id);
  if (workspaceRows.length === 0) return null;

  return buildDashboardShell(user, workspaceRows, workspaceSlug);
}

async function getDashboardShellForUser(
  user: NonNullable<Awaited<ReturnType<typeof findCurrentUser>>>,
  workspaceSlug?: string | null,
): Promise<DashboardShellData> {
  const workspaceRows = await listWorkspaceSummaries(user.id);
  if (workspaceRows.length === 0) {
    redirect('/onboarding/workspace');
  }
  return buildDashboardShell(user, workspaceRows, workspaceSlug);
}

function buildDashboardShell(
  user: NonNullable<Awaited<ReturnType<typeof findCurrentUser>>>,
  workspaceRows: Awaited<ReturnType<typeof listWorkspaceSummaries>>,
  workspaceSlug?: string | null,
): DashboardShellData {
  const selectedRow =
    workspaceRows.find((row) => row.workspace.slug === workspaceSlug) ?? workspaceRows[0]!;

  return {
    user: {
      name: user.name ?? user.email,
      email: user.email,
      avatar: user.image ?? '',
    },
    organization: selectedRow.organization,
    activeWorkspace: selectedRow.workspace,
    workspaces: workspaceRows.map((row) => row.workspace),
  };
}

export async function getWorkspaceDashboard(workspaceSlug?: string | null): Promise<WorkspaceDashboardData> {
  const shell = await getDashboardShell(workspaceSlug);
  const db = getDb();
  const workspaceId = shell.activeWorkspace.id;

  const [settingsRow] = await db
    .select()
    .from(workspaceSettings)
    .where(eq(workspaceSettings.workspaceId, workspaceId))
    .limit(1);

  const recentDecisions = await db
    .select({
      id: runtimeTraces.traceId,
      verdict: runtimeTraces.decision,
      latencyMs: runtimeTraces.elapsedMs,
      createdAt: runtimeTraces.createdAt,
      payload: runtimeTraces.payload,
    })
    .from(runtimeTraces)
    .where(eq(runtimeTraces.workspaceId, workspaceId))
    .orderBy(desc(runtimeTraces.createdAt))
    .limit(8);

  const blocked = recentDecisions.filter((decision) => decision.verdict === 'block').length;
  const escalated = recentDecisions.filter((decision) => decision.verdict === 'escalate').length;

  return {
    ...shell,
    metrics: [
      {
        label: 'Decisions',
        value: String(recentDecisions.length),
        delta: 'seeded',
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
        value: p95Latency(recentDecisions.map((decision) => Number(decision.latencyMs))),
        delta: 'demo',
        detail: 'Runtime guardrail checks',
      },
    ],
    recentDecisions: recentDecisions.map((decision) => ({
      id: String(decision.id),
      agent: readTraceAgent(decision.payload),
      verdict: decision.verdict,
      policy: readTracePolicy(decision.payload),
      latency: `${decision.latencyMs}ms`,
      time: relativeTime(decision.createdAt),
    })),
    settings: {
      defaultAction: settingsRow?.defaultAction ?? 'allow',
      escalationWebhookUrl: settingsRow?.escalationWebhookUrl ?? null,
      telemetryEnabled: settingsRow?.telemetryEnabled ?? true,
      retentionDays: settingsRow?.retentionDays ?? '30',
    },
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
  const db = getDb();
  const rows = await db
    .select({
      id: runtimeAgents.id,
      parsedProfile: runtimeAgents.parsedProfile,
    })
    .from(runtimeAgents)
    .where(and(eq(runtimeAgents.workspaceId, shell.activeWorkspace.id), isNull(runtimeAgents.deletedAt)))
    .orderBy(runtimeAgents.id);

  const policies = await listPolicyRows(shell.activeWorkspace.id);
  return {
    ...shell,
    agents: rows.map((agent) => ({
      id: agent.id,
      name: agentName(agent.parsedProfile, agent.id),
      scope: agentScope(agent.parsedProfile),
      policies: policies.filter((policy) => policy.agent === agentName(agent.parsedProfile, agent.id)).length,
      status: 'Ready',
    })),
  };
}

export async function getKnowledgePageData(
  workspaceSlug?: string | null,
): Promise<
  DashboardShellData & { knowledgeSources: KnowledgeSourceRow[] }
> {
  const shell = await getDashboardShell(workspaceSlug);
  const db = getDb();
  const rows = await db
    .select()
    .from(knowledgeSources)
    .where(and(eq(knowledgeSources.workspaceId, shell.activeWorkspace.id), isNull(knowledgeSources.deletedAt)))
    .orderBy(knowledgeSources.title);

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
      lastIndexed: source.lastIndexedAt ? relativeTime(source.lastIndexedAt) : 'Not indexed',
    })),
  };
}

export async function getApiKeysPageData(
  workspaceSlug?: string | null,
): Promise<DashboardShellData & { apiKeys: ApiKeyRow[] }> {
  const shell = await getDashboardShell(workspaceSlug);
  const db = getDb();
  const rows = await db
    .select({
      id: workspaceApiKeys.id,
      name: workspaceApiKeys.name,
      prefix: workspaceApiKeys.keyPrefix,
      status: workspaceApiKeys.status,
      lastUsedAt: workspaceApiKeys.lastUsedAt,
      createdBy: users.name,
      createdByEmail: users.email,
    })
    .from(workspaceApiKeys)
    .leftJoin(users, eq(workspaceApiKeys.createdByUserId, users.id))
    .where(eq(workspaceApiKeys.workspaceId, shell.activeWorkspace.id))
    .orderBy(workspaceApiKeys.name);

  return {
    ...shell,
    apiKeys: rows.map((row) => ({
      id: row.id,
      name: row.name,
      prefix: row.prefix,
      status: titleize(row.status),
      lastUsed: row.lastUsedAt ? relativeTime(row.lastUsedAt) : 'Never',
      createdBy: row.createdBy ?? row.createdByEmail ?? 'Unknown',
    })),
  };
}

export async function getTeamPageData(
  workspaceSlug?: string | null,
): Promise<DashboardShellData & { teamMembers: TeamMemberRow[] }> {
  const shell = await getDashboardShell(workspaceSlug);
  const db = getDb();
  const rows = await db
    .select({
      id: users.id,
      name: users.name,
      email: users.email,
      role: organizationMembers.role,
    })
    .from(organizationMembers)
    .innerJoin(users, eq(organizationMembers.userId, users.id))
    .where(eq(organizationMembers.organizationId, shell.organization.id))
    .orderBy(users.email);

  return {
    ...shell,
    teamMembers: rows.map((row) => ({
      id: row.id,
      name: row.name ?? row.email,
      email: row.email,
      role: titleize(row.role),
      access: shell.workspaces.map((workspace) => workspace.name).join(', '),
    })),
  };
}

export async function getSettingsPageData(workspaceSlug?: string | null) {
  const data = await getWorkspaceDashboard(workspaceSlug);
  return data;
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

async function getCurrentUser() {
  const user = await findCurrentUser();
  if (!user) {
    redirect('/signin');
  }
  return user;
}

async function findCurrentUser() {
  const session = await auth();
  const sessionUser = session?.user;
  if (!sessionUser?.email) {
    return null;
  }

  const db = getDb();
  const [user] = await db.select().from(users).where(eq(users.email, sessionUser.email)).limit(1);
  if (!user) {
    return null;
  }
  return user;
}

async function listWorkspaceSummaries(userId: string) {
  const db = getDb();
  const rows = await db
    .select({
      workspace: workspaces,
      workspaceRole: workspaceMembers.role,
      organization: organizations,
    })
    .from(workspaceMembers)
    .innerJoin(workspaces, eq(workspaceMembers.workspaceId, workspaces.id))
    .innerJoin(organizations, eq(workspaces.organizationId, organizations.id))
    .where(and(eq(workspaceMembers.userId, userId), isNull(workspaces.deletedAt)))
    .orderBy(workspaces.name);

  return Promise.all(
    rows.map(async (row) => {
      const [policyCount, enabledPolicyCount, agentCount, sourceCount, apiKeyCount] = await Promise.all([
        countRows(runtimePolicies, row.workspace.id),
        countRows(runtimePolicies, row.workspace.id, true),
        countRows(runtimeAgents, row.workspace.id),
        countRows(knowledgeSources, row.workspace.id),
        countRows(workspaceApiKeys, row.workspace.id),
      ]);

      return {
        organization: {
          id: row.organization.id,
          name: row.organization.name,
          slug: row.organization.slug,
        },
        workspace: {
          id: row.workspace.id,
          name: row.workspace.name,
          slug: row.workspace.slug,
          description: row.workspace.description ?? '',
          policyCount,
          enabledPolicies: enabledPolicyCount,
          agentCount,
          sourceCount,
          apiKeyCount,
          role: titleize(row.workspaceRole),
        },
      };
    }),
  );
}

async function countRows(
  table:
    | typeof runtimePolicies
    | typeof runtimeAgents
    | typeof knowledgeSources
    | typeof workspaceApiKeys,
  workspaceId: string,
  enabledOnly = false,
): Promise<number> {
  const db = getDb();
  const conditions = [eq(table.workspaceId, workspaceId)];
  if ('deletedAt' in table) {
    conditions.push(isNull(table.deletedAt));
  }
  if (enabledOnly && table === runtimePolicies) {
    conditions.push(eq(runtimePolicies.enabled, true));
  }
  const [row] = await db
    .select({ value: count() })
    .from(table)
    .where(and(...conditions));
  return row?.value ?? 0;
}

async function listPolicyRows(workspaceId: string): Promise<PolicyRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: runtimePolicies.id,
      parsedPolicy: runtimePolicies.parsedPolicy,
      enabled: runtimePolicies.enabled,
      agentId: runtimePolicies.ownerAgentId,
      agentProfile: runtimeAgents.parsedProfile,
      agentRowId: runtimeAgents.id,
    })
    .from(runtimePolicies)
    .leftJoin(
      runtimeAgents,
      and(
        eq(runtimePolicies.workspaceId, runtimeAgents.workspaceId),
        eq(runtimePolicies.ownerAgentId, runtimeAgents.id),
      ),
    )
    .where(and(eq(runtimePolicies.workspaceId, workspaceId), isNull(runtimePolicies.deletedAt)))
    .orderBy(runtimePolicies.id);

  return rows.map((row) => ({
    id: row.id,
    description: policyDescription(row.parsedPolicy),
    severity: policySeverity(row.parsedPolicy),
    action: policyAction(row.parsedPolicy),
    enabled: row.enabled,
    agent: row.agentProfile ? agentName(row.agentProfile, row.agentRowId ?? row.agentId ?? '') : 'Global',
  }));
}

async function listAgentRows(workspaceId: string, policies: PolicyRow[]): Promise<AgentRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: runtimeAgents.id,
      parsedProfile: runtimeAgents.parsedProfile,
    })
    .from(runtimeAgents)
    .where(and(eq(runtimeAgents.workspaceId, workspaceId), isNull(runtimeAgents.deletedAt)))
    .orderBy(runtimeAgents.id);

  return rows.map((agent) => ({
    id: agent.id,
    name: agentName(agent.parsedProfile, agent.id),
    scope: agentScope(agent.parsedProfile),
    policies: policies.filter((policy) => policy.agent === agentName(agent.parsedProfile, agent.id)).length,
    status: 'Ready',
  }));
}

function agentName(profile: RuntimeAgentProfile, fallback: string): string {
  return profile.display_name?.trim() || profile.agent_id?.trim() || fallback;
}

function agentScope(profile: RuntimeAgentProfile): string {
  const inScope = profile.scope?.in_scope?.filter(Boolean) ?? [];
  if (inScope.length > 0) return inScope.join(', ');
  return 'Runtime agent';
}

function policyDescription(policy: RuntimePolicyDocument): string {
  return policy.description?.trim() || 'Runtime policy';
}

function policySeverity(policy: RuntimePolicyDocument): string {
  return policy.severity ?? 'medium';
}

function policyAction(policy: RuntimePolicyDocument): string {
  return policy.action ?? 'block';
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
    .split('_')
    .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join(' ');
}
