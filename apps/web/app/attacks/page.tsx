import type { Metadata } from 'next';

import { AppLayout } from '@/components/AppLayout';
import { readParam } from '@/lib/search-params';

import { AttacksPanel } from './_components/attacks-panel';

export const metadata: Metadata = {
  title: 'Attacks | TrustLoopGuard',
  description: 'Red-team a registered agent endpoint and see what gets through.',
};

export default async function AttacksPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const environmentId = readParam(params.environment);

  return (
    <AppLayout title="Attacks" workspaceSlug={workspaceSlug} environmentId={environmentId}>
      <AttacksPanel />
    </AppLayout>
  );
}
