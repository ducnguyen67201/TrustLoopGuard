import { AppLayout } from '@/components/AppLayout';
import { WorkspaceDashboard } from '@/components/workspace/WorkspaceDashboard';
import { getWorkspaceDashboard } from '@/lib/server/dashboard-data';

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; agent?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const agentId = readParam(params.agent);
  const data = await getWorkspaceDashboard(workspaceSlug, { agentId });

  return (
    <AppLayout title="Dashboard" workspaceSlug={workspaceSlug}>
      <WorkspaceDashboard data={data} />
    </AppLayout>
  );
}

function readParam(value: string | string[] | undefined): string | null {
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
