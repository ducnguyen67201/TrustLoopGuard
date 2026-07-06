import { AppLayout } from '@/components/AppLayout';
import { FinancialApprovalsContent } from '@/components/workspace/FinancialApprovalsContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';
import type {
  FinancialActionListResponse,
  FinancialApprovalRequestListResponse,
} from '@trustloopguard/sdk';

export default async function FinancialApprovalsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const workspaceId = shell.activeWorkspace.id;
  const [approvals, actions] = await Promise.all([
    safeLoad<FinancialApprovalRequestListResponse>(
      workspaceId,
      '/v1/financial/approval-requests',
      { approval_requests: [] },
    ),
    safeLoad<FinancialActionListResponse>(workspaceId, '/v1/financial/actions', {
      actions: [],
    }),
  ]);

  return (
    <AppLayout
      title="Financial approvals"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <FinancialApprovalsContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        approvals={approvals.approval_requests}
        actions={actions.actions}
      />
    </AppLayout>
  );
}

async function safeLoad<T>(workspaceId: string, path: string, fallback: T): Promise<T> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' });
  } catch (error) {
    console.error('[financial approvals] failed to load', path, error);
    return fallback;
  }
}
