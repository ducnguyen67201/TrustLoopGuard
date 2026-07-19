import { notFound } from 'next/navigation';

import { AppLayout } from '@/components/AppLayout';
import { McpAccessPageContent } from '@/components/workspace/McpAccessPageContent';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getMcpAccessPageData } from '@/lib/server/dashboard-data';
import { isWorkspaceFeatureEnabled } from '@/lib/workspace-features';

export default async function McpAccessPage({ searchParams }: { searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }> }) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const data = await getMcpAccessPageData(workspaceSlug, environmentId);
  if (!isWorkspaceFeatureEnabled(data.activeWorkspace, 'mcpAccess')) notFound();
  return <AppLayout title="MCP Access" workspaceSlug={workspaceSlug} environmentId={environmentId} shell={data}><McpAccessPageContent data={data} /></AppLayout>;
}
