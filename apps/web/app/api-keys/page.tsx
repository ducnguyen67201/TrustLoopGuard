import { AppLayout } from '@/components/AppLayout';
import { ApiKeysPageContent } from '@/components/workspace/ApiKeysPageContent';
import { getApiKeysPageData } from '@/lib/server/dashboard-data';

export default async function ApiKeysPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getApiKeysPageData(workspaceSlug);

  return (
    <AppLayout title="API Keys" workspaceSlug={workspaceSlug}>
      <ApiKeysPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
