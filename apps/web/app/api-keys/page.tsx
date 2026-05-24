import { AppLayout } from '@/components/AppLayout';
import { ApiKeysPageContent } from '@/components/workspace/ApiKeysPageContent';
import { getApiKeysPageData } from '@/lib/server/dashboard-data';

export default async function ApiKeysPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const data = await getApiKeysPageData(workspaceSlug, environmentId);

  return (
    <AppLayout title="API Keys" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <ApiKeysPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}

function readParam(value: string | string[] | undefined): string | null {
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
