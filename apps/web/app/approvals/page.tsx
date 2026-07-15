import type { AuthorizationApprovalListResponse } from '@trustloopguard/sdk';

import { AppLayout } from '@/components/AppLayout';
import { AuthorizationApprovalsContent } from '@/components/workspace/AuthorizationApprovalsContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForUserWorkspace } from '@/lib/server/tl-client';

export default async function ApprovalsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  let response: AuthorizationApprovalListResponse = { approvals: [] };
  try {
    response = await rustApiForUserWorkspace<AuthorizationApprovalListResponse>(
      shell.user,
      shell.activeWorkspace.id,
      '/v1/authorization/approvals',
      { method: 'GET' },
      shell.activeEnvironment.id,
    );
  } catch (error) {
    console.error('[approvals] failed to load authorization queue', error);
  }

  return (
    <AppLayout
      title="Approvals"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <AuthorizationApprovalsContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        approvals={response.approvals}
      />
    </AppLayout>
  );
}
