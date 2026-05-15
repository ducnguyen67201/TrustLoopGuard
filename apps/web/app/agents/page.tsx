import { AppLayout } from '@/components/AppLayout';
import { AgentsPageContent } from '@/components/workspace/ManagementPages';
import { getAgentsPageData } from '@/lib/server/dashboard-data';

export default async function AgentsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getAgentsPageData(workspaceSlug);

  return (
    <AppLayout title="Agents" workspaceSlug={workspaceSlug}>
      <AgentsPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
