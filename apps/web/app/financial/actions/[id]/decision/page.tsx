import { notFound } from 'next/navigation';
import type { FinancialActionDecisionReceipt, FinancialActionRecord } from '@trustloopguard/sdk';

import { AppLayout } from '@/components/AppLayout';
import { FinancialDecisionReceiptContent } from '@/components/workspace/FinancialDecisionReceiptContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';

export default async function FinancialDecisionReceiptPage({
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
  const workspaceId = shell.activeWorkspace.id;
  const receipt = await safeLoad<FinancialActionDecisionReceipt>(
    workspaceId,
    `/v1/financial/actions/${encodeURIComponent(id)}/decision-receipt`,
    null,
    shell.activeEnvironment.id,
  );
  if (receipt === null) notFound();
  const action = await safeLoad<FinancialActionRecord>(
    workspaceId,
    `/v1/financial/actions/${encodeURIComponent(receipt.action_id)}`,
    null,
    shell.activeEnvironment.id,
  );

  return (
    <AppLayout
      title="Financial decision receipt"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <FinancialDecisionReceiptContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        receipt={receipt}
        action={action}
      />
    </AppLayout>
  );
}

async function safeLoad<T>(
  workspaceId: string,
  path: string,
  fallback: T | null,
  environmentId?: string | null,
): Promise<T | null> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' }, environmentId);
  } catch (error) {
    console.error('[financial decision receipt] failed to load', path, error);
    return fallback;
  }
}
