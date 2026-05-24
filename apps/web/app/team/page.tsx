import { AppLayout } from '@/components/AppLayout';
import { TeamPageContent } from '@/components/workspace/ManagementPages';
import { readWorkspaceSlug } from '@/lib/search-params';
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
