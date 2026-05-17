import { AppLayout } from '@/components/AppLayout';
import { AnalyticsPageContent } from '@/components/workspace/ManagementPages';
import {
  getAnalyticsPageData,
  type RunAnalyticsFilterParams,
} from '@/lib/server/dashboard-data';

type AnalyticsSearchParams = {
  workspace?: string | string[];
  agent?: string | string[];
  agent_id?: string | string[];
  status?: string | string[];
  kind?: string | string[];
  externalId?: string | string[];
  external_id?: string | string[];
  limit?: string | string[];
};

export default async function AnalyticsPage({
  searchParams,
}: {
  searchParams: Promise<AnalyticsSearchParams>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const filters: RunAnalyticsFilterParams = {
    agentId: readParam(params.agent_id) ?? readParam(params.agent),
    status: readParam(params.status),
    kind: readParam(params.kind),
    externalId: readParam(params.external_id) ?? readParam(params.externalId),
    limit: readParam(params.limit),
  };
  const data = await getAnalyticsPageData(workspaceSlug, filters);

  return (
    <AppLayout title="Analytics" workspaceSlug={workspaceSlug} shell={data}>
      <AnalyticsPageContent data={data} />
    </AppLayout>
  );
}

function readParam(value: string | string[] | undefined): string | null {
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
