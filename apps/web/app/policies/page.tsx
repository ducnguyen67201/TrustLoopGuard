import { AppLayout } from '@/components/AppLayout';
import { PoliciesPageContent } from '@/components/workspace/PoliciesPageContent';
import { readParam } from '@/lib/search-params';
import { getPoliciesPageData } from '@/lib/server/dashboard-data';

export default async function PoliciesPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; agent?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const agentId = readParam(params.agent);
  const environmentId = readParam(params.environment);
  const data = await getPoliciesPageData(workspaceSlug, { agentId, environmentId });

  return (
    <AppLayout title="Policies" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}>
      <PoliciesPageContent data={data} />
    </AppLayout>
  );
}
