import { AppLayout } from '@/components/AppLayout';
import { WorkspacesPageContent } from '@/components/workspace/ManagementPages';
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

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
