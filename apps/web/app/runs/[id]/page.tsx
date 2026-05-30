import { AppLayout } from '@/components/AppLayout';
import { RunDetailPageContent } from '@/components/workspace/ManagementPages';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getRunDetailPageData } from '@/lib/server/dashboard-data';

export default async function RunDetailPage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const [{ id }, resolvedSearchParams] = await Promise.all([params, searchParams]);
  const workspaceSlug = readWorkspaceSlug(resolvedSearchParams);
  const environmentId = readParam(resolvedSearchParams.environment);
  const data = await getRunDetailPageData(id, workspaceSlug, environmentId);

  return (
    <AppLayout title="Run detail" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <RunDetailPageContent data={data} />
    </AppLayout>
  );
}
