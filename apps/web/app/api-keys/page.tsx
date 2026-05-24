import { AppLayout } from '@/components/AppLayout';
import { ApiKeysPageContent } from '@/components/workspace/ApiKeysPageContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getApiKeysPageData } from '@/lib/server/dashboard-data';

export default async function ApiKeysPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const data = await getApiKeysPageData(workspaceSlug, environmentId);

  return (
    <AppLayout title="API Keys" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <ApiKeysPageContent data={data} />
    </AppLayout>
  );
}
