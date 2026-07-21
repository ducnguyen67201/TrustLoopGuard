import type {
  AuthorizationApprovalListResponse,
  AuthorizationReceiptListResponse,
} from '@trustloopguard/sdk';

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
  const approvalsRequest = rustApiForUserWorkspace<AuthorizationApprovalListResponse>(
    shell.user,
    shell.activeWorkspace.id,
    '/v1/authorization/approvals',
    { method: 'GET' },
    shell.activeEnvironment.id,
  );
  const receiptsRequest = rustApiForUserWorkspace<AuthorizationReceiptListResponse>(
    shell.user,
    shell.activeWorkspace.id,
    '/v1/authorization/receipts',
    { method: 'GET' },
    shell.activeEnvironment.id,
  );
  const [approvalsResult, receiptsResult] = await Promise.allSettled([
    approvalsRequest,
    receiptsRequest,
  ]);
  if (approvalsResult.status === 'rejected') {
    console.error('[authorization] failed to load approval queue', approvalsResult.reason);
  }
  if (receiptsResult.status === 'rejected') {
    console.error('[authorization] failed to load receipt activity', receiptsResult.reason);
  }
  const approvals = approvalsResult.status === 'fulfilled' ? approvalsResult.value.approvals : [];
  const receipts = receiptsResult.status === 'fulfilled' ? receiptsResult.value.receipts : [];

  return (
    <AppLayout
      title="Authorization"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <AuthorizationApprovalsContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        approvals={approvals}
        receipts={receipts}
      />
    </AppLayout>
  );
}
