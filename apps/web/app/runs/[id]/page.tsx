import { AppLayout } from '@/components/AppLayout';
import { RunDetailPageContent } from '@/components/workspace/ManagementPages';
import { getRunDetailPageData } from '@/lib/server/dashboard-data';

export default async function RunDetailPage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const [{ id }, resolvedSearchParams] = await Promise.all([params, searchParams]);
  const workspaceSlug = readWorkspaceSlug(resolvedSearchParams);
  const data = await getRunDetailPageData(id, workspaceSlug);

  return (
    <AppLayout title="Run detail" workspaceSlug={workspaceSlug}>
      <RunDetailPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
