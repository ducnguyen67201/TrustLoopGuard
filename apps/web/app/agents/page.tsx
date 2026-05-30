import { AppLayout } from '@/components/AppLayout';
import { AgentsPageContent } from '@/components/workspace/ManagementPages';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getAgentsPageData } from '@/lib/server/dashboard-data';

export default async function AgentsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const data = await getAgentsPageData(workspaceSlug, environmentId);

  return (
    <AppLayout title="Agents" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <AgentsPageContent data={data} />
    </AppLayout>
  );
}
