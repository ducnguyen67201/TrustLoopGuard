import { AppLayout } from '@/components/AppLayout';
import { SettingsPageContent } from '@/components/workspace/ManagementPages';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getSettingsPageData } from '@/lib/server/dashboard-data';

export default async function SettingsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const data = await getSettingsPageData(workspaceSlug, environmentId);

  return (
    <AppLayout title="Settings" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <SettingsPageContent data={data} />
    </AppLayout>
  );
}
