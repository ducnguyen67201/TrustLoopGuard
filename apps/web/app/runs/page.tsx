import { AppLayout } from '@/components/AppLayout';
import { RunsPageContent } from '@/components/workspace/ManagementPages';
import { getRunsPageData } from '@/lib/server/dashboard-data';

export default async function RunsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getRunsPageData(workspaceSlug);

  return (
    <AppLayout title="Runs" workspaceSlug={workspaceSlug}>
      <RunsPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
