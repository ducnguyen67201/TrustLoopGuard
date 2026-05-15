import { AppLayout } from '@/components/AppLayout';
import { KnowledgeSourcesPageContent } from '@/components/workspace/ManagementPages';
import { getKnowledgePageData } from '@/lib/server/dashboard-data';

export default async function KnowledgeSourcesPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getKnowledgePageData(workspaceSlug);

  return (
    <AppLayout title="Knowledge" workspaceSlug={workspaceSlug}>
      <KnowledgeSourcesPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
