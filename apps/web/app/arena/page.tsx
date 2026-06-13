import { redirect } from 'next/navigation';

import { readParam } from '@/lib/search-params';

/**
 * The Arena page was removed — its durable successor is the Attacks tab, which
 * dispatches Rust-owned red-team jobs (`/v1/redteam/*`). This thin redirect keeps
 * old `/arena` links working by forwarding to `/attacks`, preserving the workspace
 * and environment context the old page read.
 */
export default async function ArenaRedirectPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const environmentId = readParam(params.environment);

  const query = new URLSearchParams();
  if (workspaceSlug) query.set('workspace', workspaceSlug);
  if (environmentId) query.set('environment', environmentId);

  const queryString = query.toString();
  redirect(queryString ? `/attacks?${queryString}` : '/attacks');
}
