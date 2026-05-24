import { AppLayout } from '@/components/AppLayout';
import { WorkspacesPageContent } from '@/components/workspace/ManagementPages';
import { readWorkspaceSlug } from '@/lib/search-params';
import { getWorkspacesPageData } from '@/lib/server/dashboard-data';

export default async function WorkspacesPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getWorkspacesPageData(workspaceSlug);

  return (
    <AppLayout title="Workspaces" workspaceSlug={workspaceSlug}>
      <WorkspacesPageContent data={data} />
    </AppLayout>
  );
}
