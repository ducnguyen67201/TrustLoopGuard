import { AppLayout } from '@/components/AppLayout';
import { PoliciesPageContent } from '@/components/workspace/PoliciesPageContent';
import { getPoliciesPageData } from '@/lib/server/dashboard-data';

export default async function PoliciesPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getPoliciesPageData(workspaceSlug);

  return (
    <AppLayout title="Policies" shell={data}>
      <PoliciesPageContent data={data} />
    </AppLayout>
  );
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
