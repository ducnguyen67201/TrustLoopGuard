import { AppLayout } from '@/components/AppLayout';
import { UsageContent } from '@/components/workspace/UsageContent';
import { loadUsageBuckets } from '@/components/workspace/usage/loader';
import { readUsagePeriod } from '@/components/workspace/usage';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';

export default async function UsagePage({
  searchParams,
}: {
  searchParams: Promise<{
    workspace?: string | string[];
    environment?: string | string[];
    period?: string | string[];
    principal?: string | string[];
  }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const period = readUsagePeriod(readParam(params.period));
  const principalId = readParam(params.principal);
  const shell = await getDashboardShell(workspaceSlug, environmentId);

  const buckets = await loadUsageBuckets(shell.activeWorkspace.id, { period, principalId });

  return (
    <AppLayout
      title="Usage"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <UsageContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        buckets={buckets}
      />
    </AppLayout>
  );
}
