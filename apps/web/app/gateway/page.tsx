import { AppLayout } from '@/components/AppLayout';
import { GatewayPageContent } from '@/components/workspace/GatewayPageContent';
import { env } from '@/env';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getGatewayPageData } from '@/lib/server/dashboard-data';
import type { FamilyPolicyRow } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';
import type { BudgetAlertConfigListResponse, LlmPricingListResponse } from '@featherlane-ai/sdk';

export default async function GatewayPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const data = await getGatewayPageData(workspaceSlug, environmentId);
  const workspaceId = data.activeWorkspace.id;
  const activeEnvironmentId = data.activeEnvironment.id;
  const [pricing, policies, alerts] = await Promise.all([
    safeLoad<LlmPricingListResponse>(workspaceId, activeEnvironmentId, '/v1/llm-pricing', {
      prices: [],
    }),
    safeLoad<{ policies: FamilyPolicyRow[] }>(
      workspaceId,
      activeEnvironmentId,
      '/v1/financial/policies',
      { policies: [] },
    ),
    safeLoad<BudgetAlertConfigListResponse>(
      workspaceId,
      activeEnvironmentId,
      '/v1/financial/budget-alerts',
      { configs: [] },
    ),
  ]);
  const routedModels = data.gatewayRoutes.flatMap((route) => {
    const provider = data.providerConnections.find(
      (candidate) => candidate.id === route.provider_connection_id,
    );
    return provider ? [provider.default_model] : [];
  });

  return (
    <AppLayout
      title="Gateway"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={data}
    >
      <GatewayPageContent
        data={data}
        apiBaseUrl={env.NEXT_PUBLIC_TL_SERVER_URL}
        budgetReadiness={{
          hasPrice: routedModels.some((model) => hasEffectivePrice(pricing, model)),
          hasCap: policies.policies.some((policy) => policy.meter === 'llm_usage'),
          hasAlert: alerts.configs.some((config) => config.meter === 'llm_usage'),
        }}
      />
    </AppLayout>
  );
}

function hasEffectivePrice(pricing: LlmPricingListResponse, model: string): boolean {
  const normalized = model.trim().toLowerCase();
  return pricing.prices.some(
    (price) =>
      normalized === price.model ||
      normalized.endsWith(`/${price.model}`) ||
      normalized.endsWith(`:${price.model}`),
  );
}

async function safeLoad<T>(
  workspaceId: string,
  environmentId: string,
  path: string,
  fallback: T,
): Promise<T> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' }, environmentId);
  } catch (error) {
    console.error('[gateway] failed to load budget readiness', path, error);
    return fallback;
  }
}
