import { AppLayout } from '@/components/AppLayout';
import { TeamPageContent } from '@/components/workspace/ManagementPages';
import { getTeamPageData } from '@/lib/server/dashboard-data';

export default async function TeamPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getTeamPageData(workspaceSlug);

  return (
    <AppLayout title="Team" workspaceSlug={workspaceSlug}>
      <TeamPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
