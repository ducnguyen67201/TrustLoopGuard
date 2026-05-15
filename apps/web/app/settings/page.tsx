import { AppLayout } from '@/components/AppLayout';
import { SettingsPageContent } from '@/components/workspace/ManagementPages';
import { getSettingsPageData } from '@/lib/server/dashboard-data';

export default async function SettingsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getSettingsPageData(workspaceSlug);

  return (
    <AppLayout title="Settings" workspaceSlug={workspaceSlug}>
      <SettingsPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
