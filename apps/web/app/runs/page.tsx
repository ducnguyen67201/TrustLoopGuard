import { AppLayout } from '@/components/AppLayout';
import { RunsPageContent } from '@/components/workspace/ManagementPages';
import { readParam } from '@/lib/search-params';
import { getRunsPageData } from '@/lib/server/dashboard-data';

export default async function RunsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; agent?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const agentId = readParam(params.agent);
  const environmentId = readParam(params.environment);
  const data = await getRunsPageData(workspaceSlug, { agentId, environmentId });

  return (
    <AppLayout title="Runs" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <RunsPageContent data={data} />
    </AppLayout>
  );
}
