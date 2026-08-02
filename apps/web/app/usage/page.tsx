import type {
  BudgetAlertConfig,
  BudgetAlertConfigListResponse,
  BudgetAlertFiring,
  BudgetAlertFiringListResponse,
} from '@featherlane-ai/sdk';

import { AppLayout } from '@/components/AppLayout';
import { BudgetAlertsCard } from '@/components/workspace/BudgetAlertsCard';
import { UsageBudgetsCard } from '@/components/workspace/UsageBudgetsCard';
import { UsageContent } from '@/components/workspace/UsageContent';
import { readUsagePeriod } from '@/components/workspace/usage-utils';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell, type FamilyPolicyRow } from '@/lib/server/dashboard-data';
import { getLlmUsageDashboardData } from '@/lib/server/llm-usage-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';

export default async function UsagePage({
  searchParams,
}: {
  searchParams: Promise<{
    workspace?: string | string[];
    environment?: string | string[];
    period?: string | string[];
  }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const period = readUsagePeriod(readParam(params.period));
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const workspaceId = shell.activeWorkspace.id;
  const [customerUsage, guardrailUsage, policies, alerts] = await Promise.all([
    getLlmUsageDashboardData(workspaceId, period, 'customer_inference'),
    getLlmUsageDashboardData(workspaceId, period, 'guardrail'),
    safeLoad<{ policies: FamilyPolicyRow[] }>(
      workspaceId,
      '/v1/financial/policies',
      { policies: [] },
      shell.activeEnvironment.id,
    ),
    safeLoad<BudgetAlertConfigListResponse>(workspaceId, '/v1/financial/budget-alerts', {
      configs: [],
    }),
  ]);
  const llmAlerts = alerts.configs.filter((config) => config.meter === 'llm_usage');
  const firings = await loadFirings(workspaceId, llmAlerts);
  const contextQuery = contextQueryFor(shell.activeWorkspace.slug, shell.activeEnvironment.id);

  return (
    <AppLayout
      title="Usage & budgets"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <div className="grid gap-6">
        <UsageContent
          workspaceSlug={shell.activeWorkspace.slug}
          environmentId={shell.activeEnvironment.id}
          period={period}
          dayBuckets={customerUsage.dayBuckets}
          principalBuckets={customerUsage.principalBuckets}
          modelBuckets={customerUsage.modelBuckets}
          guardrailBuckets={guardrailUsage.modelBuckets}
        />
        <UsageBudgetsCard contextQuery={contextQuery} policies={policies.policies} />
        <BudgetAlertsCard
          contextQuery={contextQuery}
          configs={llmAlerts}
          firings={firings}
          meter="llm_usage"
        />
      </div>
    </AppLayout>
  );
}

function contextQueryFor(workspaceSlug: string, environmentId: string): string {
  const params = new URLSearchParams({ workspace: workspaceSlug, environment: environmentId });
  return `?${params.toString()}`;
}

async function loadFirings(
  workspaceId: string,
  configs: BudgetAlertConfig[],
): Promise<BudgetAlertFiring[]> {
  const results = await Promise.all(
    configs.map((config) =>
      safeLoad<BudgetAlertFiringListResponse>(
        workspaceId,
        `/v1/financial/budget-alerts/${encodeURIComponent(config.id)}/firings`,
        { firings: [] },
      ),
    ),
  );
  return results
    .flatMap((result) => result.firings)
    .sort((a, b) => b.fired_at.localeCompare(a.fired_at));
}

async function safeLoad<T>(
  workspaceId: string,
  path: string,
  fallback: T,
  environmentId?: string,
): Promise<T> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' }, environmentId);
  } catch (error) {
    console.error('[usage] failed to load', path, error);
    return fallback;
  }
}
