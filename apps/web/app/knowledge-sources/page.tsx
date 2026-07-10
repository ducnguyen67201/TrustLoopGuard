import { AppLayout } from '@/components/AppLayout';
import { notFound } from 'next/navigation';
import { KnowledgeSourcesPageContent } from '@/components/workspace/ManagementPages';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getKnowledgePageData } from '@/lib/server/dashboard-data';
import { isWorkspaceFeatureEnabled } from '@/lib/workspace-features';

export default async function KnowledgeSourcesPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const data = await getKnowledgePageData(workspaceSlug, environmentId);
  if (!isWorkspaceFeatureEnabled(data.activeWorkspace, 'knowledgeBase')) notFound();

  return (
    <AppLayout title="Knowledge" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <KnowledgeSourcesPageContent data={data} />
    </AppLayout>
  );
}
