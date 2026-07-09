import { AppLayout } from '@/components/AppLayout';
import { WorkspaceDashboard } from '@/components/workspace/WorkspaceDashboard';
import { readUsagePeriod } from '@/components/workspace/usage-utils';
import { readParam } from '@/lib/search-params';
import { getWorkspaceDashboard } from '@/lib/server/dashboard-data';
import { getLlmUsageDashboardData } from '@/lib/server/llm-usage-data';

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<{
    workspace?: string | string[];
    agent?: string | string[];
    environment?: string | string[];
    period?: string | string[];
  }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const agentId = readParam(params.agent);
  const environmentId = readParam(params.environment);
  const usagePeriod = readUsagePeriod(readParam(params.period));
  const data = await getWorkspaceDashboard(workspaceSlug, { agentId, environmentId });
  const usage = await getLlmUsageDashboardData(data.activeWorkspace.id, usagePeriod);

  return (
    <AppLayout title="Dashboard" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <WorkspaceDashboard data={data} usage={usage} usagePeriod={usagePeriod} />
    </AppLayout>
  );
}
