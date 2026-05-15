import { AppLayout } from '@/components/AppLayout';
import { WorkspaceDashboard } from '@/components/workspace/WorkspaceDashboard';
import { getWorkspaceDashboard } from '@/lib/server/dashboard-data';

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getWorkspaceDashboard(workspaceSlug);

  return (
    <AppLayout title="Dashboard" workspaceSlug={workspaceSlug}>
      <WorkspaceDashboard data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
