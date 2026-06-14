import type { Metadata } from 'next';

import { AppLayout } from '@/components/AppLayout';
import { readParam } from '@/lib/search-params';

import { InternalAgentDemoPageContent } from './page-content';

export const metadata: Metadata = {
  title: 'Internal Agent Demo | TrustLoopGuard',
};

export default async function InternalAgentDemoPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const environmentId = readParam(params.environment);

  return (
    <AppLayout title="Internal Agent Demo" workspaceSlug={workspaceSlug} environmentId={environmentId}>
      <InternalAgentDemoPageContent />
    </AppLayout>
  );
}
