import 'server-only';

import { and, count, desc, eq, isNull } from 'drizzle-orm';
import { redirect } from 'next/navigation';

import { auth } from '@/auth';
import { getDb } from '@/lib/db/client';
import { users } from '@/lib/db/schema/auth';
import {
  guardrailDecisions,
  knowledgeSources,
  organizationMembers,
  organizations,
  workspaceAgents,
  workspaceApiKeys,
  workspaceMembers,
  workspacePolicies,
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
  const workspaceRows = await listWorkspaceSummaries(user.id);
  if (workspaceRows.length === 0) {
    redirect('/onboarding/workspace');
  }
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
      id: guardrailDecisions.traceId,
      verdict: guardrailDecisions.verdict,
      latencyMs: guardrailDecisions.latencyMs,
      createdAt: guardrailDecisions.createdAt,
      agent: workspaceAgents.name,
      policy: workspacePolicies.policyKey,
    })
    .from(guardrailDecisions)
    .leftJoin(workspaceAgents, eq(guardrailDecisions.agentId, workspaceAgents.id))
    .leftJoin(workspacePolicies, eq(guardrailDecisions.policyId, workspacePolicies.id))
    .where(eq(guardrailDecisions.workspaceId, workspaceId))
    .orderBy(desc(guardrailDecisions.createdAt))
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
      id: decision.id,
      agent: decision.agent ?? 'Unknown agent',
      verdict: decision.verdict,
      policy: decision.policy ?? 'baseline',
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
      id: workspaceAgents.id,
      name: workspaceAgents.name,
      scope: workspaceAgents.scope,
      status: workspaceAgents.status,
    })
    .from(workspaceAgents)
    .where(and(eq(workspaceAgents.workspaceId, shell.activeWorkspace.id), isNull(workspaceAgents.deletedAt)))
    .orderBy(workspaceAgents.name);

  const policies = await listPolicyRows(shell.activeWorkspace.id);
  return {
    ...shell,
    agents: rows.map((agent) => ({
      ...agent,
      policies: policies.filter((policy) => policy.agent === agent.name).length,
      status: titleize(agent.status),
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
): Promise<DashboardShellData & { policies: PolicyRow[] }> {
  const shell = await getDashboardShell(workspaceSlug);
  return {
    ...shell,
    policies: await listPolicyRows(shell.activeWorkspace.id),
  };
}

async function getCurrentUser() {
  const session = await auth();
  const sessionUser = session?.user;
  if (!sessionUser?.email) {
    redirect('/signin');
  }

  const db = getDb();
  const [user] = await db.select().from(users).where(eq(users.email, sessionUser.email)).limit(1);
  if (!user) {
    redirect('/signin');
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
        countRows(workspacePolicies, row.workspace.id),
        countRows(workspacePolicies, row.workspace.id, true),
        countRows(workspaceAgents, row.workspace.id),
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
    | typeof workspacePolicies
    | typeof workspaceAgents
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
  if (enabledOnly && table === workspacePolicies) {
    conditions.push(eq(workspacePolicies.enabled, true));
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
      id: workspacePolicies.id,
      policyKey: workspacePolicies.policyKey,
      description: workspacePolicies.description,
      severity: workspacePolicies.severity,
      action: workspacePolicies.action,
      enabled: workspacePolicies.enabled,
      agent: workspaceAgents.name,
    })
    .from(workspacePolicies)
    .leftJoin(workspaceAgents, eq(workspacePolicies.agentId, workspaceAgents.id))
    .where(and(eq(workspacePolicies.workspaceId, workspaceId), isNull(workspacePolicies.deletedAt)))
    .orderBy(workspacePolicies.policyKey);

  return rows.map((row) => ({
    id: row.policyKey,
    description: row.description,
    severity: row.severity,
    action: row.action,
    enabled: row.enabled,
    agent: row.agent ?? 'Global',
  }));
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
