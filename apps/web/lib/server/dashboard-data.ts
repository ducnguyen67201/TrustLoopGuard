import 'server-only';

import { cookies } from 'next/headers';
import { redirect } from 'next/navigation';

import { auth } from '@/auth';
import { getAppUrl } from '@/env';
import { analyticsCatalogSchema, analyticsDashboardViewListSchema } from '@/lib/analytics-schemas';
import { http } from '@/lib/http';
import { runDetailSnapshot, type RunDetailSnapshot } from '@/lib/run-detail-live';
import {
  isUserApprovalRequiredError,
  normalizeWorkspaceSlug,
  rustApiForUser,
  rustApiForUserWorkspace,
  rustApiForWorkspace,
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
  activeEnvironment: WorkspaceEnvironmentSummary;
  environments: WorkspaceEnvironmentSummary[];
  agents: { id: string; name: string }[];
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

export interface WorkspaceEnvironmentSummary {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  isDefault: boolean;
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
    environment: string;
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
  environment: string;
  status: string;
  lastUsed: string;
  createdBy: string;
  principal: string | null;
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
};

export type PolicyRow = {
  id: string;
  family: string;
  description: string;
  severity: string;
  action: string;
  enabled: boolean;
  agent: string;
};

export type RunRow = {
  id: string;
  shortId: string;
  agent: string;
  environment: string;
  kind: string;
  status: string;
  externalId: string;
  traces: number;
  blocked: number;
  rewritten: number;
  escalated: number;
  p95LatencyMs: number | null;
  latency: string;
  started: string;
  startedAt: string;
  endedAt: string;
  metadata: Array<{ label: string; value: string }>;
  href: string;
};

export type RunTraceRow = {
  id: string;
  runEventId: string | null;
  environment: string;
  verdict: string;
  latestReviewOutcome: string;
  policy: string;
  latency: string;
  time: string;
};

export type RunEventRow = {
  id: string;
  sequence: number;
  kind: string;
  label: string;
  input: string;
  output: string;
  time: string;
  metadata: Array<{ label: string; value: string }>;
};

export type RunAnalyticsFilterParams = {
  agentId?: string | null;
  status?: string | null;
  kind?: string | null;
  externalId?: string | null;
  limit?: string | null;
};

type HumanReviewAnalytics = {
  summary: {
    traceCount: number;
    automatedInterventionCount: number;
    humanReviewCount: number;
    humanInterventionCount: number;
    humanInterventionRate: number;
    falsePositiveRate: number;
  };
  outcomes: {
    acceptedCount: number;
    correctedCount: number;
    rejectedCount: number;
    falsePositiveCount: number;
    missedIssueCount: number;
    ignoredCount: number;
  };
  byWorkflowStep: Array<{
    workflowStep: string;
    humanReviewCount: number;
    correctedCount: number;
    rejectedCount: number;
    falsePositiveCount: number;
  }>;
  byPolicy: Array<{
    policyId: string;
    escalationCount: number;
    correctedCount: number;
    falsePositiveCount: number;
  }>;
  byAgent: Array<{ group: string; humanReviewCount: number; humanInterventionCount: number }>;
  byRunKind: Array<{ group: string; humanReviewCount: number; humanInterventionCount: number }>;
  topReasons: Array<{ reasonCode: string; count: number }>;
};

export type AnalyticsMetric =
  | 'trace_count'
  | 'allow_count'
  | 'block_count'
  | 'rewrite_count'
  | 'escalate_count'
  | 'intervention_rate'
  | 'p95_latency_ms'
  | 'human_review_count'
  | 'human_intervention_rate'
  | 'false_positive_rate';

export type AnalyticsDimension =
  | 'agent_id'
  | 'environment'
  | 'run_kind'
  | 'run_status'
  | 'decision'
  | 'policy_id'
  | 'workflow_step'
  | 'review_outcome'
  | 'external_id';

export type AnalyticsChartType = 'big_number' | 'bar' | 'line' | 'area' | 'donut' | 'table';

export type AnalyticsFilter = {
  dimension: AnalyticsDimension;
  values: string[];
};

export type AnalyticsWidgetLayout = {
  x: number;
  y: number;
  w: number;
  h: number;
};

export type AnalyticsDashboardWidget = {
  id: string;
  title: string;
  metric: AnalyticsMetric;
  chart_type: AnalyticsChartType;
  group_by?: AnalyticsDimension | null | undefined;
  layout?: AnalyticsWidgetLayout | undefined;
};

export type AnalyticsDashboardViewConfig = {
  filters: AnalyticsFilter[];
  widgets: AnalyticsDashboardWidget[];
};

export type AnalyticsDashboardView = {
  id: string;
  name: string;
  is_default: boolean;
  config: AnalyticsDashboardViewConfig;
  created_at: string;
  updated_at: string;
};

export type AnalyticsCatalog = {
  metrics: Array<{
    metric: AnalyticsMetric;
    label: string;
    default_chart_type: AnalyticsChartType;
  }>;
  dimensions: Array<{ dimension: AnalyticsDimension; label: string }>;
  chart_types: AnalyticsChartType[];
  facets: Array<{ dimension: AnalyticsDimension; label: string; values: string[] }>;
};

export type AnalyticsQueryRequest = {
  metric: AnalyticsMetric;
  group_by?: AnalyticsDimension | null;
  filters: AnalyticsFilter[];
  limit?: number;
};

export type AnalyticsQueryResponse = {
  metric: AnalyticsMetric;
  group_by?: AnalyticsDimension | null | undefined;
  total: number;
  points: Array<{ label: string; value: number }>;
};

type CurrentUser = {
  id: string;
  name: string;
  email: string;
  image: string;
  /// Rust-issued JWT from the credentials sign-in flow. Absent for
  /// OAuth users (Google/GitHub) until OAuth ↔ Rust-user binding
  /// lands; their requests fall back to header-forwarded identity.
  tlJwt?: string | undefined;
};

export class UserApprovalRequiredError extends Error {
  constructor() {
    super('user is not approved for dashboard access');
  }
}

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
  family?: string;
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
  run_id?: string | null;
  run_event_id?: string | null;
  environment_id: string;
  environment: string;
  domain: string;
  decision: string;
  elapsed_ms: number;
  latest_review_outcome?: string | null;
  latest_reviewed_at?: string | null;
  payload: RuntimeDecisionPayload;
  created_at: string;
};

type TraceListWire = {
  traces: TraceSummaryWire[];
};

type RunSummaryWire = {
  id: string;
  workspace_id: string;
  environment_id: string;
  environment: string;
  agent_id: string;
  kind: string;
  status: string;
  external_id: string | null;
  metadata: Record<string, unknown>;
  started_at: string;
  ended_at: string | null;
  created_at: string;
  updated_at: string;
  trace_count: number;
  blocked_count: number;
  rewritten_count: number;
  escalated_count: number;
  p95_latency_ms: number | null;
};

type RunListWire = {
  runs: RunSummaryWire[];
};

type RunEventSummaryWire = {
  id: string;
  workspace_id: string;
  run_id: string;
  sequence: number;
  kind: string;
  label: string | null;
  input_summary: string | null;
  output_summary: string | null;
  metadata: Record<string, unknown>;
  occurred_at: string;
  created_at: string;
};

type RunDetailWire = {
  run: RunSummaryWire;
  events: RunEventSummaryWire[];
  traces: TraceSummaryWire[];
};

type HumanReviewAnalyticsWire = {
  summary: {
    trace_count: number;
    automated_intervention_count: number;
    human_review_count: number;
    human_intervention_count: number;
    human_intervention_rate: number;
    false_positive_rate: number;
  };
  outcomes: {
    accepted_count: number;
    corrected_count: number;
    rejected_count: number;
    false_positive_count: number;
    missed_issue_count: number;
    ignored_count: number;
  };
  by_workflow_step: Array<{
    workflow_step: string;
    human_review_count: number;
    corrected_count: number;
    rejected_count: number;
    false_positive_count: number;
  }>;
  by_policy: Array<{
    policy_id: string;
    escalation_count: number;
    corrected_count: number;
    false_positive_count: number;
  }>;
  by_agent: Array<{ group: string; human_review_count: number; human_intervention_count: number }>;
  by_run_kind: Array<{
    group: string;
    human_review_count: number;
    human_intervention_count: number;
  }>;
  top_reasons: Array<{ reason_code: string; count: number }>;
};

type AnalyticsDashboardViewListWire = {
  views: AnalyticsDashboardView[];
};

type ApiKeyWire = {
  id: string;
  name: string;
  prefix: string;
  environment_id: string;
  environment: string;
  status: string;
  created_at: string;
  last_used_at: string | null;
  created_by: string | null;
  principal_id: string | null;
};

type ApiKeyListWire = {
  api_keys: ApiKeyWire[];
};

type WorkspaceEnvironmentWire = {
  id: string;
  slug: string;
  name: string;
  description?: string | null;
  is_default: boolean;
  created_at: string;
  updated_at: string;
};

type WorkspaceEnvironmentListWire = {
  environments: WorkspaceEnvironmentWire[];
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

export async function getDashboardShell(
  workspaceSlug?: string | null,
  environmentId?: string | null,
): Promise<DashboardShellData> {
  const user = await getCurrentUser();
  return buildDashboardShell(user, workspaceSlug, environmentId);
}

export async function getOptionalDashboardShell(
  workspaceSlug?: string | null,
  environmentId?: string | null,
): Promise<DashboardShellData | null> {
  const user = await findCurrentUser();
  if (!user) return null;
  return buildDashboardShell(user, workspaceSlug, environmentId);
}

export async function getWorkspaceDashboard(
  workspaceSlug?: string | null,
  filters: { agentId?: string | null; environmentId?: string | null } = {},
): Promise<WorkspaceDashboardData> {
  const shell = await getDashboardShell(workspaceSlug, filters.environmentId);
  const workspaceId = shell.activeWorkspace.id;
  const { agentId } = filters;
  const traceParams = new URLSearchParams({ limit: agentId ? '50' : '8' });
  if (agentId) traceParams.set('agent_id', agentId);
  const allTraces = (
    await rustApiForWorkspace<TraceListWire>(
      workspaceId,
      `/v1/traces?${traceParams}`,
      {},
      shell.activeEnvironment.id,
    )
  ).traces;
  const recentDecisions = agentId
    ? allTraces.filter((t) => t.payload?.agent_id === agentId).slice(0, 8)
    : allTraces;
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
      environment: decision.environment,
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
  environmentId?: string | null,
): Promise<DashboardShellData & { agents: AgentRow[] }> {
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const policies = await listPolicyRows(shell.activeWorkspace.id, shell.activeEnvironment.id);
  return {
    ...shell,
    agents: await listAgentRows(shell.activeWorkspace.id, policies),
  };
}

export async function getKnowledgePageData(
  workspaceSlug?: string | null,
  environmentId?: string | null,
): Promise<DashboardShellData & { knowledgeSources: KnowledgeSourceRow[] }> {
  const shell = await getDashboardShell(workspaceSlug, environmentId);
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
      lastIndexed: source.last_indexed_at
        ? relativeTime(new Date(source.last_indexed_at))
        : 'Not indexed',
    })),
  };
}

export async function getApiKeysPageData(
  workspaceSlug?: string | null,
  environmentId?: string | null,
): Promise<DashboardShellData & { apiKeys: ApiKeyRow[] }> {
  const user = await getCurrentUser();
  const shell = await buildDashboardShell(user, workspaceSlug, environmentId);
  const rows = (
    await rustApiForUserWorkspace<ApiKeyListWire>(
      user,
      shell.activeWorkspace.id,
      '/v1/api-keys',
      {},
      shell.activeEnvironment.id,
    )
  ).api_keys;
  return {
    ...shell,
    apiKeys: rows.map((key) => ({
      id: key.id,
      name: key.name,
      prefix: key.prefix,
      environment: key.environment,
      status: titleize(key.status),
      lastUsed: key.last_used_at ? relativeTime(new Date(key.last_used_at)) : 'Never',
      createdBy: key.created_by ?? 'System',
      principal: key.principal_id,
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
): Promise<DashboardShellData & { teamMembers: TeamMemberRow[]; invites: TeamInviteRow[] }> {
  const shell = await getDashboardShell(workspaceSlug);
  const workspaceId = shell.activeWorkspace.id;

  const [members, invites] = await Promise.all([
    rustApiForWorkspace<{ members: RustMember[] }>(workspaceId, '/v1/team/members').catch(() => ({
      members: [] as RustMember[],
    })),
    rustApiForWorkspace<{ invites: RustInvite[] }>(workspaceId, '/v1/team/invites').catch(() => ({
      invites: [] as RustInvite[],
    })),
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
  }));

  return {
    ...shell,
    teamMembers,
    invites: inviteRows,
  };
}

export async function getSettingsPageData(
  workspaceSlug?: string | null,
  environmentId?: string | null,
) {
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  return {
    ...shell,
    metrics: [],
    recentDecisions: [],
    settings: settingsFromWire(await getWorkspaceSettings(shell.activeWorkspace.id)),
  };
}

/**
 * A financial policy as served by `GET /v1/financial/policies`. Financial
 * controls are stored in the unified policy registry, but this endpoint returns
 * the domain-specific cap fields the Financial page needs.
 */
export type FamilyPolicyRow = {
  id: string;
  description?: string | null;
  severity?: string;
  when?: {
    agents?: string[];
    action_kinds?: string[];
    operations?: string[];
    currencies?: string[];
    rails?: string[];
  };
  meter?: string;
  per_transaction_minor?: number | null;
  hold_above_minor?: number | null;
  daily_minor?: number | null;
  weekly_minor?: number | null;
  monthly_minor?: number | null;
  allowed_counterparty_ids?: string[];
  denied_counterparty_ids?: string[];
  hold_new_counterparty?: boolean;
  mandate_required?: boolean;
  approval_threshold_minor?: number | null;
  approver_roles?: string[];
  refund_original_method_only?: boolean;
  required_preconditions?: string[];
  missing_evidence_action?: string;
  failed_precondition_action?: string;
  on_breach?: string;
  enabled?: boolean;
};

async function listFamilyPolicyRows(
  workspaceId: string,
  environmentId: string,
): Promise<FamilyPolicyRow[]> {
  try {
    const wire = await rustApiForWorkspace<{ policies: FamilyPolicyRow[] }>(
      workspaceId,
      '/v1/financial/policies',
      {},
      environmentId,
    );
    return wire.policies;
  } catch (error) {
    // Family policies are additive context on this page — degrade to none
    // rather than failing the whole policies view.
    console.error('family policy list failed', error);
    return [];
  }
}

export async function getPoliciesPageData(
  workspaceSlug?: string | null,
  filters: { agentId?: string | null; environmentId?: string | null } = {},
): Promise<
  DashboardShellData & {
    agents: AgentRow[];
    policies: PolicyRow[];
    familyPolicies: FamilyPolicyRow[];
  }
> {
  const shell = await getDashboardShell(workspaceSlug, filters.environmentId);
  const [policies, familyPolicies] = await Promise.all([
    listPolicyRows(shell.activeWorkspace.id, shell.activeEnvironment.id, filters.agentId),
    listFamilyPolicyRows(shell.activeWorkspace.id, shell.activeEnvironment.id),
  ]);
  return {
    ...shell,
    agents: await listAgentRows(shell.activeWorkspace.id, policies),
    policies,
    familyPolicies,
  };
}

export async function getRunsPageData(
  workspaceSlug?: string | null,
  filters: { agentId?: string | null; environmentId?: string | null } = {},
): Promise<DashboardShellData & { runs: RunRow[] }> {
  const shell = await getDashboardShell(workspaceSlug, filters.environmentId);
  const rows = (
    await rustApiForWorkspace<RunListWire>(
      shell.activeWorkspace.id,
      `/v1/runs${runAnalyticsQuery({ agentId: filters.agentId ?? null, limit: '50' })}`,
      {},
      shell.activeEnvironment.id,
    )
  ).runs;
  return {
    ...shell,
    runs: rows.map((run) => runRow(run, shell.activeWorkspace.slug)),
  };
}

export async function getAnalyticsPageData(
  workspaceSlug?: string | null,
  filters: RunAnalyticsFilterParams & { environmentId?: string | null } = {},
): Promise<
  DashboardShellData & {
    analyticsCatalog: AnalyticsCatalog;
    analyticsViews: AnalyticsDashboardView[];
  }
> {
  const shell = await getDashboardShell(workspaceSlug, filters.environmentId);
  const apiHeaders = await sameOriginApiHeaders();
  const [analyticsCatalog, analyticsViews] = await Promise.all([
    http.withoutWorkspace.get(
      sameOriginApiUrl(
        '/api/analytics/catalog',
        shell.activeWorkspace.slug,
        shell.activeEnvironment.id,
      ),
      analyticsCatalogSchema,
      { headers: apiHeaders, cache: 'no-store' },
    ),
    http.withoutWorkspace
      .get(
        sameOriginApiUrl(
          '/api/analytics/views',
          shell.activeWorkspace.slug,
          shell.activeEnvironment.id,
        ),
        analyticsDashboardViewListSchema,
        { headers: apiHeaders, cache: 'no-store' },
      )
      .catch(() => ({ views: [] as AnalyticsDashboardView[] })),
  ]);
  return {
    ...shell,
    analyticsCatalog,
    analyticsViews: analyticsViews.views,
  };
}

export async function getRunDetailPageData(
  runId: string,
  workspaceSlug?: string | null,
  environmentId?: string | null,
): Promise<
  DashboardShellData & {
    run: RunRow;
    events: RunEventRow[];
    traces: RunTraceRow[];
    liveSnapshot: RunDetailSnapshot;
  }
> {
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const detail = await rustApiForWorkspace<RunDetailWire>(
    shell.activeWorkspace.id,
    `/v1/runs/${encodeURIComponent(runId)}`,
    {},
    shell.activeEnvironment.id,
  );
  return {
    ...shell,
    run: runRow(detail.run, shell.activeWorkspace.slug),
    events: detail.events.map(eventRow),
    traces: detail.traces.map(traceRow),
    liveSnapshot: runDetailSnapshot(detail),
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

  const tlJwt = (sessionUser as { tlJwt?: string }).tlJwt?.trim();

  return {
    id: sessionUser.id,
    name: username,
    email: sessionUser.email?.trim() || username,
    image: sessionUser.image ?? '',
    tlJwt: tlJwt !== undefined && tlJwt !== '' ? tlJwt : undefined,
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
export async function getMyWorkspaces(
  user: CurrentUser,
  opts: { throwOnApprovalRequired?: boolean } = {},
): Promise<MyWorkspaceWire[]> {
  try {
    const data = await rustApiForUser<MyWorkspacesWire>(
      { id: user.id, email: user.email },
      '/v1/team/my-workspaces',
    );
    return data.workspaces;
  } catch (err) {
    if (isUserApprovalRequiredError(err)) {
      if (opts.throwOnApprovalRequired === true) throw new UserApprovalRequiredError();
      return [];
    }
    return [];
  }
}

export async function getWorkspaceAccessState(
  user: CurrentUser,
): Promise<{ kind: 'ready'; workspaces: MyWorkspaceWire[] } | { kind: 'pending_approval' }> {
  try {
    return {
      kind: 'ready',
      workspaces: await getMyWorkspaces(user, { throwOnApprovalRequired: true }),
    };
  } catch (err) {
    if (err instanceof UserApprovalRequiredError) return { kind: 'pending_approval' };
    throw err;
  }
}

export async function requireApprovedOnboardingUser(): Promise<CurrentUser> {
  const user = await getCurrentUser();
  const access = await getWorkspaceAccessState(user);
  if (access.kind === 'pending_approval') redirect('/welcome');
  return user;
}

async function buildDashboardShell(
  user: CurrentUser,
  workspaceSlug?: string | null,
  environmentId?: string | null,
): Promise<DashboardShellData> {
  const access = await getWorkspaceAccessState(user);
  if (access.kind === 'pending_approval') redirect('/welcome');
  const memberships = access.workspaces;
  if (memberships.length === 0) {
    // No workspace = nothing to render: approved users start first-run
    // onboarding. Pages that render in user-state-agnostic shells
    // (e.g. /welcome itself) call getOptionalDashboardShell instead and
    // avoid this branch.
    redirect('/onboarding/workspace');
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

  const environments = await listWorkspaceEnvironments(active.id);
  const activeEnvironment = selectActiveEnvironment(environments, environmentId);
  const agentListWire = await rustApiForWorkspace<AgentListWire>(active.id, '/v1/agents').catch(
    () => ({ agents: [] as AgentProfileWire[] }),
  );
  const agents = agentListWire.agents.map((a) => ({
    id: a.agent_id,
    name: agentName(a, a.agent_id),
  }));

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
    activeEnvironment,
    environments,
    agents,
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

async function listWorkspaceEnvironments(
  workspaceId: string,
): Promise<WorkspaceEnvironmentSummary[]> {
  const data = await rustApiForWorkspace<WorkspaceEnvironmentListWire>(
    workspaceId,
    '/v1/environments',
  ).catch(() => ({
    environments: [
      {
        id: 'production',
        slug: 'production',
        name: 'Production',
        description: null,
        is_default: true,
        created_at: '',
        updated_at: '',
      },
    ],
  }));

  return data.environments.map((environment) => ({
    id: environment.id,
    slug: environment.slug,
    name: environment.name,
    description: environment.description ?? null,
    isDefault: environment.is_default,
  }));
}

function selectActiveEnvironment(
  environments: WorkspaceEnvironmentSummary[],
  environmentId?: string | null,
): WorkspaceEnvironmentSummary {
  const requested = environmentId?.trim();
  if (requested) {
    const match = environments.find(
      (environment) => environment.id === requested || environment.slug === requested,
    );
    if (match) return match;
  }
  return (
    environments.find((environment) => environment.isDefault) ??
    environments.find((environment) => environment.id === 'production') ??
    environments[0] ?? {
      id: 'production',
      slug: 'production',
      name: 'Production',
      description: null,
      isDefault: true,
    }
  );
}

async function listPolicyRows(
  workspaceId: string,
  environmentId: string,
  agentId?: string | null,
): Promise<PolicyRow[]> {
  const [policyList, agentList] = await Promise.all([
    rustApiForWorkspace<PolicyListWire>(workspaceId, '/v1/policies', {}, environmentId),
    rustApiForWorkspace<AgentListWire>(workspaceId, '/v1/agents'),
  ]);
  const agentsById = new Map(agentList.agents.map((agent) => [agent.agent_id, agent]));
  const filteredPolicies = agentId
    ? policyList.policies.filter((p) => p.owner_agent_id === agentId)
    : policyList.policies;

  return filteredPolicies.map((policy) => ({
    id: policy.id,
    family: policy.family ?? 'content',
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

function runRow(run: RunSummaryWire, workspaceSlug: string): RunRow {
  const params = new URLSearchParams({ workspace: workspaceSlug, environment: run.environment_id });
  return {
    id: run.id,
    shortId: shortRunId(run.id),
    agent: run.agent_id,
    environment: run.environment,
    kind: titleize(run.kind),
    status: titleize(run.status),
    externalId: run.external_id?.trim() || 'None',
    traces: run.trace_count,
    blocked: run.blocked_count,
    rewritten: run.rewritten_count,
    escalated: run.escalated_count,
    p95LatencyMs: run.p95_latency_ms,
    latency: run.p95_latency_ms === null ? 'No traces' : `${run.p95_latency_ms}ms`,
    started: relativeTime(new Date(run.started_at)),
    startedAt: formatDateTime(new Date(run.started_at)),
    endedAt: run.ended_at ? formatDateTime(new Date(run.ended_at)) : 'Still running',
    metadata: metadataEntries(run.metadata),
    href: `/runs/${encodeURIComponent(run.id)}?${params.toString()}`,
  };
}

function runAnalyticsQuery(filters: RunAnalyticsFilterParams): string {
  const params = new URLSearchParams();
  params.set('limit', normalizeLimit(filters.limit));
  appendQueryParam(params, 'agent_id', filters.agentId);
  appendQueryParam(params, 'status', filters.status);
  appendQueryParam(params, 'kind', filters.kind);
  appendQueryParam(params, 'external_id', filters.externalId);
  return `?${params.toString()}`;
}

function humanReviewAnalyticsQuery(filters: RunAnalyticsFilterParams): string {
  const params = new URLSearchParams();
  appendQueryParam(params, 'agent_id', filters.agentId);
  appendQueryParam(params, 'run_kind', filters.kind);
  const serialized = params.toString();
  return serialized === '' ? '' : `?${serialized}`;
}

function appendQueryParam(params: URLSearchParams, key: string, value: string | null | undefined) {
  const clean = value?.trim();
  if (clean) params.set(key, clean);
}

function normalizeLimit(value: string | null | undefined): string {
  const parsed = Number.parseInt(value ?? '', 10);
  if (!Number.isFinite(parsed)) return '50';
  return String(Math.min(100, Math.max(1, parsed)));
}

async function sameOriginApiHeaders(): Promise<HeadersInit> {
  const cookieHeader = (await cookies()).toString();
  return cookieHeader === '' ? {} : { cookie: cookieHeader };
}

function sameOriginApiUrl(path: string, workspaceSlug: string, environmentId: string): string {
  const params = new URLSearchParams({ workspace: workspaceSlug, environment: environmentId });
  return `${getAppUrl()}${path}?${params.toString()}`;
}

function humanReviewAnalyticsFromWire(wire: HumanReviewAnalyticsWire): HumanReviewAnalytics {
  return {
    summary: {
      traceCount: wire.summary.trace_count,
      automatedInterventionCount: wire.summary.automated_intervention_count,
      humanReviewCount: wire.summary.human_review_count,
      humanInterventionCount: wire.summary.human_intervention_count,
      humanInterventionRate: wire.summary.human_intervention_rate,
      falsePositiveRate: wire.summary.false_positive_rate,
    },
    outcomes: {
      acceptedCount: wire.outcomes.accepted_count,
      correctedCount: wire.outcomes.corrected_count,
      rejectedCount: wire.outcomes.rejected_count,
      falsePositiveCount: wire.outcomes.false_positive_count,
      missedIssueCount: wire.outcomes.missed_issue_count,
      ignoredCount: wire.outcomes.ignored_count,
    },
    byWorkflowStep: wire.by_workflow_step.map((row) => ({
      workflowStep: row.workflow_step,
      humanReviewCount: row.human_review_count,
      correctedCount: row.corrected_count,
      rejectedCount: row.rejected_count,
      falsePositiveCount: row.false_positive_count,
    })),
    byPolicy: wire.by_policy.map((row) => ({
      policyId: row.policy_id,
      escalationCount: row.escalation_count,
      correctedCount: row.corrected_count,
      falsePositiveCount: row.false_positive_count,
    })),
    byAgent: wire.by_agent.map((row) => ({
      group: row.group,
      humanReviewCount: row.human_review_count,
      humanInterventionCount: row.human_intervention_count,
    })),
    byRunKind: wire.by_run_kind.map((row) => ({
      group: row.group,
      humanReviewCount: row.human_review_count,
      humanInterventionCount: row.human_intervention_count,
    })),
    topReasons: wire.top_reasons.map((row) => ({
      reasonCode: row.reason_code,
      count: row.count,
    })),
  };
}

function traceRow(trace: TraceSummaryWire): RunTraceRow {
  return {
    id: trace.trace_id,
    runEventId: trace.run_event_id ?? null,
    environment: trace.environment,
    verdict: titleize(trace.decision),
    latestReviewOutcome: trace.latest_review_outcome
      ? titleize(trace.latest_review_outcome)
      : 'Not reviewed',
    policy: readTracePolicy(trace.payload),
    latency: `${trace.elapsed_ms}ms`,
    time: relativeTime(new Date(trace.created_at)),
  };
}

function eventRow(event: RunEventSummaryWire): RunEventRow {
  return {
    id: event.id,
    sequence: event.sequence,
    kind: titleize(event.kind),
    label: event.label?.trim() || defaultEventLabel(event.kind, event.sequence),
    input: event.input_summary?.trim() || 'No input summary',
    output: event.output_summary?.trim() || 'No output summary',
    time: relativeTime(new Date(event.occurred_at)),
    metadata: metadataEntries(event.metadata),
  };
}

function defaultEventLabel(kind: string, sequence: number): string {
  if (kind === 'user_turn' || kind === 'assistant_turn') return `Turn ${sequence}`;
  if (kind === 'workflow_step') return `Step ${sequence}`;
  return `Event ${sequence}`;
}

function shortRunId(id: string): string {
  if (id.length <= 12) return id;
  return `${id.slice(0, 8)}...${id.slice(-4)}`;
}

function metadataEntries(
  metadata: Record<string, unknown>,
): Array<{ label: string; value: string }> {
  return Object.entries(metadata)
    .filter(([, value]) => value !== null && value !== undefined && value !== '')
    .map(([key, value]) => ({
      label: titleize(key),
      value: formatMetadataValue(value),
    }));
}

function formatMetadataValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

function formatDateTime(date: Date): string {
  return date.toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
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
