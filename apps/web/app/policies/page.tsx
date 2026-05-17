import { AppLayout } from '@/components/AppLayout';
import { PoliciesPageContent } from '@/components/workspace/PoliciesPageContent';
import { getPoliciesPageData } from '@/lib/server/dashboard-data';

export default async function PoliciesPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; agent?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const agentId = readParam(params.agent);
  const data = await getPoliciesPageData(workspaceSlug, { agentId });

  return (
    <AppLayout title="Policies" shell={data}>
      <PoliciesPageContent data={data} />
    </AppLayout>
  );
}

function readParam(value: string | string[] | undefined): string | null {
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
