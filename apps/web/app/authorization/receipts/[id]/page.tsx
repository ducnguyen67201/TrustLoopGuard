import type { AuthorizationReceipt } from '@trustloopguard/sdk';
import { notFound } from 'next/navigation';

import { AppLayout } from '@/components/AppLayout';
import { AuthorizationReceiptContent } from '@/components/workspace/AuthorizationReceiptContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { RustApiError, rustApiForUserWorkspace } from '@/lib/server/tl-client';

export default async function AuthorizationReceiptPage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const [{ id }, query] = await Promise.all([params, searchParams]);
  const workspaceSlug = readWorkspaceSlug(query);
  const environmentId = readParam(query.environment);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const path = `/v1/authorization/receipts/${encodeURIComponent(id)}`;
  let receipt: AuthorizationReceipt;
  try {
    receipt = await rustApiForUserWorkspace<AuthorizationReceipt>(
      shell.user,
      shell.activeWorkspace.id,
      path,
      { method: 'GET' },
      shell.activeEnvironment.id,
    );
  } catch (error) {
    if (error instanceof RustApiError && error.status === 404) {
      notFound();
    }
    console.error('[authorization receipt] failed to load', path, error);
    throw error;
  }

  return (
    <AppLayout
      title="Authorization receipt"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <AuthorizationReceiptContent
        receipt={receipt}
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
      />
    </AppLayout>
  );
}
