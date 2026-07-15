import { notFound } from 'next/navigation';

import { AppLayout } from '@/components/AppLayout';
import { FinancialReceiptContent } from '@/components/workspace/FinancialReceiptContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';
import type {
  FinancialActionRecord,
  FinancialOutcomeListResponse,
  FinancialReceipt,
} from '@trustloopguard/sdk';

export default async function FinancialReceiptPage({
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
  const receipt = await safeLoad<FinancialReceipt>(
    workspaceId,
    `/v1/financial/receipts/${encodeURIComponent(id)}`,
    null,
    shell.activeEnvironment.id,
  );
  if (receipt === null) notFound();
  const [action, outcomes] = await Promise.all([
    safeLoad<FinancialActionRecord>(
      workspaceId,
      `/v1/financial/actions/${encodeURIComponent(receipt.action_id)}`,
      null,
      shell.activeEnvironment.id,
    ),
    safeLoad<FinancialOutcomeListResponse>(
      workspaceId,
      `/v1/financial/actions/${encodeURIComponent(receipt.action_id)}/outcomes`,
      { outcomes: [] },
      shell.activeEnvironment.id,
    ),
  ]);

  return (
    <AppLayout
      title="Financial receipt"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <FinancialReceiptContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        receipt={receipt}
        action={action}
        outcomes={outcomes?.outcomes ?? []}
      />
    </AppLayout>
  );
}

async function safeLoad<T>(
  workspaceId: string,
  path: string,
  fallback: T | null,
  environmentId: string,
): Promise<T | null> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' }, environmentId);
  } catch (error) {
    console.error('[financial receipt] failed to load', path, error);
    return fallback;
  }
}
