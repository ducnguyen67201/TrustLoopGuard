import { AppLayout } from '@/components/AppLayout';
import { WorkspaceDashboard } from '@/components/workspace/WorkspaceDashboard';
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

function readParam(value: string | string[] | undefined): string | null {
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
