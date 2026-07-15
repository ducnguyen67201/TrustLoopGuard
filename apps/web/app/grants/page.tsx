import type { AuthorizationGrantListResponse } from '@trustloopguard/sdk';

import { AppLayout } from '@/components/AppLayout';
import { AuthorizationGrantsContent } from '@/components/workspace/AuthorizationGrantsContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';

export default async function GrantsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  let response: AuthorizationGrantListResponse = { grants: [] };
  try {
    response = await rustApiForWorkspace<AuthorizationGrantListResponse>(
      shell.activeWorkspace.id,
      '/v1/authorization/grants',
      { method: 'GET' },
      shell.activeEnvironment.id,
    );
  } catch (error) {
    console.error('[grants] failed to load grants', error);
  }

  return (
    <AppLayout
      title="Grants"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <AuthorizationGrantsContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        grants={response.grants}
      />
    </AppLayout>
  );
}
