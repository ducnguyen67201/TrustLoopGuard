import { AppLayout } from '@/components/AppLayout';
import { FinancialActionsContent } from '@/components/workspace/FinancialActionsContent';
import { loadUsageBuckets } from '@/components/workspace/usage/loader';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell, type FamilyPolicyRow } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';
import type {
  FinancialActionListResponse,
  FinancialActionOutcome,
  FinancialApprovalRequestListResponse,
  FinancialOutcomeListResponse,
  GatewayProviderConnectionListResponse,
} from '@trustloopguard/sdk';

export default async function FinancialPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const workspaceId = shell.activeWorkspace.id;

  const [actions, approvals, familyPolicies, providers] = await Promise.all([
    safeLoad<FinancialActionListResponse>(workspaceId, '/v1/financial/actions', {
      actions: [],
    }),
    safeLoad<FinancialApprovalRequestListResponse>(workspaceId, '/v1/financial/approval-requests', {
      approval_requests: [],
    }),
    safeLoad<{ policies: FamilyPolicyRow[] }>(
      workspaceId,
      '/v1/financial/policies',
      { policies: [] },
      shell.activeEnvironment.id,
    ),
    safeLoad<GatewayProviderConnectionListResponse>(
      workspaceId,
      '/v1/gateway/provider-connections',
      { provider_connections: [] },
    ),
  ]);
  const outcomesByActionId = await loadOutcomesByActionId(workspaceId, actions.actions);
  // Token spend is money, so it lives on the money page: one loader call, no by-day
  // series needed for the compact section.
  const usage = await loadUsageBuckets(workspaceId, {
    period: 'week',
    groups: ['principal'],
  });

  return (
    <AppLayout
      title="Financial actions"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <FinancialActionsContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        actions={actions.actions}
        approvals={approvals.approval_requests}
        outcomesByActionId={outcomesByActionId}
        familyPolicies={familyPolicies.policies}
        providerConnections={providers.provider_connections}
        usagePrincipalBuckets={usage.principal}
      />
    </AppLayout>
  );
}

async function loadOutcomesByActionId(
  workspaceId: string,
  actions: FinancialActionListResponse['actions'],
): Promise<Record<string, FinancialActionOutcome[]>> {
  const entries = await Promise.all(
    actions.map(async (action) => {
      const outcomes = await safeLoad<FinancialOutcomeListResponse>(
        workspaceId,
        `/v1/financial/actions/${encodeURIComponent(action.id)}/outcomes`,
        { outcomes: [] },
      );
      return [action.id, outcomes.outcomes] as const;
    }),
  );
  return Object.fromEntries(entries);
}

async function safeLoad<T>(
  workspaceId: string,
  path: string,
  fallback: T,
  environmentId?: string | null,
): Promise<T> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' }, environmentId);
  } catch (error) {
    console.error('[financial] failed to load', path, error);
    return fallback;
  }
}
