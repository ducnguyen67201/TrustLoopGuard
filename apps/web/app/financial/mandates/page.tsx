import { AppLayout } from '@/components/AppLayout';
import { FinancialMandatesContent } from '@/components/workspace/FinancialMandatesContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';
import type { FinancialMandateListResponse } from '@trustloopguard/sdk';

export default async function FinancialMandatesPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const mandates = await safeLoad<FinancialMandateListResponse>(
    shell.activeWorkspace.id,
    '/v1/financial/mandates',
    { mandates: [] },
  );

  return (
    <AppLayout
      title="Financial mandates"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <FinancialMandatesContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        mandates={mandates.mandates}
      />
    </AppLayout>
  );
}

async function safeLoad<T>(workspaceId: string, path: string, fallback: T): Promise<T> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' });
  } catch (error) {
    console.error('[financial mandates] failed to load', path, error);
    return fallback;
  }
}
