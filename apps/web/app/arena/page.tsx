import type { Metadata } from 'next';

import { AppLayout } from '@/components/AppLayout';
import { readParam } from '@/lib/search-params';

import { ArenaPageContent } from './page-content';

export const metadata: Metadata = {
  title: 'Arena | TrustLoopGuard',
  description: 'Compare a raw agent with a TrustLoopGuard-protected agent.',
};

export default async function ArenaPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const environmentId = readParam(params.environment);

  return (
    <AppLayout title="Arena" workspaceSlug={workspaceSlug} environmentId={environmentId}>
      <ArenaPageContent />
    </AppLayout>
  );
}
