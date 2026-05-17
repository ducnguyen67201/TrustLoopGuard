import { AppLayout } from '@/components/AppLayout';
import { RunsPageContent } from '@/components/workspace/ManagementPages';
import { getRunsPageData } from '@/lib/server/dashboard-data';

export default async function RunsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; agent?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const agentId = readParam(params.agent);
  const data = await getRunsPageData(workspaceSlug, { agentId });

  return (
    <AppLayout title="Runs" workspaceSlug={workspaceSlug}>
      <RunsPageContent data={data} />
    </AppLayout>
  );
}

function readParam(value: string | string[] | undefined): string | null {
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
