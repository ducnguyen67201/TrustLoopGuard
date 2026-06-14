import type { Metadata } from 'next';

import { AppLayout } from '@/components/AppLayout';
import { readParam } from '@/lib/search-params';

import { BenchPanel } from './_components/bench-panel';

export const metadata: Metadata = {
  title: 'Bench | TrustLoopGuard',
  description: 'Run durable raw-vs-guarded TrustLoopGuardBench comparisons.',
};

export default async function BenchPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const environmentId = readParam(params.environment);

  return (
    <AppLayout title="Bench" workspaceSlug={workspaceSlug} environmentId={environmentId}>
      <BenchPanel />
    </AppLayout>
  );
}
