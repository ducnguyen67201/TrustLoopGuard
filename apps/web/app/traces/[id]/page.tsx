import { AppLayout } from '@/components/AppLayout';
import { TraceDetailPageContent } from '@/components/workspace/TraceDetailPageContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getTraceDetailPageData } from '@/lib/server/dashboard-data';

export default async function TraceDetailPage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const [{ id }, resolvedSearchParams] = await Promise.all([params, searchParams]);
  const workspaceSlug = readWorkspaceSlug(resolvedSearchParams);
  const environmentId = readParam(resolvedSearchParams.environment);
  const data = await getTraceDetailPageData(id, workspaceSlug, environmentId);

  const homeParams = new URLSearchParams({ workspace: data.activeWorkspace.slug });
  if (environmentId) homeParams.set('environment', environmentId);

  return (
    <AppLayout
      title="Trace replay"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={data}
      breadcrumbs={[
        { label: 'Dashboard', href: `/?${homeParams.toString()}` },
        { label: data.trace.trace_id },
      ]}
    >
      <TraceDetailPageContent data={data} />
    </AppLayout>
  );
}
