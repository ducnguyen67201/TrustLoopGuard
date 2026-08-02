import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { AppLayout } from '@/components/AppLayout';
import { readParam } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { isWorkspaceFeatureEnabled } from '@/lib/workspace-features';

import { AttacksPanel } from './_components/attacks-panel';

export const metadata: Metadata = {
  title: 'Attacks | Featherlane AI',
  description: 'Red-team a registered agent endpoint and see what gets through.',
};

export default async function AttacksPage({
  searchParams,
}: {
  searchParams: Promise<{
    workspace?: string | string[];
    environment?: string | string[];
    id?: string | string[];
  }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const environmentId = readParam(params.environment);
  const initialJobId = readParam(params.id);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  if (!isWorkspaceFeatureEnabled(shell.activeWorkspace, 'attacks')) notFound();

  return (
    <AppLayout
      title="Attacks"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <AttacksPanel initialJobId={initialJobId} />
    </AppLayout>
  );
}
