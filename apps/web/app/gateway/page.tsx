import { AppLayout } from '@/components/AppLayout';
import { GatewayPageContent } from '@/components/workspace/GatewayPageContent';
import { env } from '@/env';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getGatewayPageData } from '@/lib/server/dashboard-data';

export default async function GatewayPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const data = await getGatewayPageData(workspaceSlug, environmentId);

  return (
    <AppLayout title="Gateway" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <GatewayPageContent data={data} apiBaseUrl={env.NEXT_PUBLIC_TL_SERVER_URL} />
    </AppLayout>
  );
}
