import { AppLayout } from '@/components/AppLayout';
import { GatewayPageContent } from '@/components/workspace/GatewayPageContent';
import { env } from '@/env';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';
import type {
  ApiKeyListResponse,
  EnforcementProfileListResponse,
  GatewayProviderConnectionListResponse,
  GatewayRouteListResponse,
} from '@trustloopguard/sdk';

export default async function GatewayPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const workspaceId = shell.activeWorkspace.id;

  const [providers, profiles, routes, apiKeys] = await Promise.all([
    safeLoad<GatewayProviderConnectionListResponse>(
      workspaceId,
      '/v1/gateway/provider-connections',
      { provider_connections: [] },
    ),
    safeLoad<EnforcementProfileListResponse>(
      workspaceId,
      '/v1/enforcement-profiles',
      { enforcement_profiles: [] },
    ),
    safeLoad<GatewayRouteListResponse>(workspaceId, '/v1/gateway/routes', {
      gateway_routes: [],
    }),
    safeLoad<ApiKeyListResponse>(workspaceId, '/v1/api-keys', { api_keys: [] }),
  ]);

  return (
    <AppLayout title="Gateway" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={shell}>
      <GatewayPageContent
        data={{
          ...shell,
          providerConnections: providers.provider_connections,
          enforcementProfiles: profiles.enforcement_profiles,
          gatewayRoutes: routes.gateway_routes,
          activeRuntimeKeyCount: apiKeys.api_keys.filter((key) => key.status === 'active').length,
        }}
        apiBaseUrl={env.NEXT_PUBLIC_TL_SERVER_URL}
      />
    </AppLayout>
  );
}

async function safeLoad<T>(workspaceId: string, path: string, fallback: T): Promise<T> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' });
  } catch (err) {
    console.error('[gateway] failed to load', path, err);
    return fallback;
  }
}
