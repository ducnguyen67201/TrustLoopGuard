import { redirect } from 'next/navigation';

import { readParam, readWorkspaceSlug } from '@/lib/search-params';

export default async function FinancialApprovalsPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const query = new URLSearchParams();
  const workspaceSlug = readWorkspaceSlug(params);
  if (workspaceSlug) {
    query.set('workspace', workspaceSlug);
  }
  const environmentId = readParam(params.environment);
  if (environmentId) {
    query.set('environment', environmentId);
  }
  const suffix = query.toString();
  redirect(suffix ? `/financial?${suffix}` : '/financial');
}
