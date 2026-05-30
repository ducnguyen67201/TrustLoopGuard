import { AppLayout } from '@/components/AppLayout';
import { WorkspaceDashboard } from '@/components/workspace/WorkspaceDashboard';
import { readParam } from '@/lib/search-params';
import { getWorkspaceDashboard } from '@/lib/server/dashboard-data';

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; agent?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const agentId = readParam(params.agent);
  const environmentId = readParam(params.environment);
  const data = await getWorkspaceDashboard(workspaceSlug, { agentId, environmentId });

  return (
    <AppLayout title="Dashboard" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <WorkspaceDashboard data={data} />
    </AppLayout>
  );
}
