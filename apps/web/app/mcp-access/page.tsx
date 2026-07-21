import { notFound } from 'next/navigation';

import { AppLayout } from '@/components/AppLayout';
import { McpAccessPageContent } from '@/components/workspace/McpAccessPageContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell, getMcpAccessPageData } from '@/lib/server/dashboard-data';
import { isWorkspaceFeatureEnabled } from '@/lib/workspace-features';

export default async function McpAccessPage({ searchParams }: { searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[]; member?: string | string[] }> }) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const memberId = readParam(params.member);
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  if (!isWorkspaceFeatureEnabled(shell.activeWorkspace, 'mcpAccess')) notFound();
  const data = await getMcpAccessPageData(workspaceSlug, environmentId);
  return <AppLayout title="MCP Access" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}><McpAccessPageContent data={data} initialMemberId={memberId ?? undefined} /></AppLayout>;
}
