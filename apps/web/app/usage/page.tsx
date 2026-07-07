import { AppLayout } from '@/components/AppLayout';
import { UsageContent } from '@/components/workspace/UsageContent';
import { periodRange, readUsagePeriod } from '@/components/workspace/usage-utils';
import { readParam, readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';
import type { LlmUsageBucketsResponse } from '@trustloopguard/sdk';

export default async function UsagePage({
  searchParams,
}: {
  searchParams: Promise<{
    workspace?: string | string[];
    environment?: string | string[];
    period?: string | string[];
  }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const environmentId = readParam(params.environment);
  const period = readUsagePeriod(readParam(params.period));
  const shell = await getDashboardShell(workspaceSlug, environmentId);
  const workspaceId = shell.activeWorkspace.id;

  const { start, end } = periodRange(period, new Date());
  const window = `start=${encodeURIComponent(start.toISOString())}&end=${encodeURIComponent(end.toISOString())}`;
  const load = (groupBy: 'day' | 'principal' | 'model') =>
    safeLoad(workspaceId, `/v1/llm-usage?group_by=${groupBy}&${window}`);
  const [byDay, byPrincipal, byModel] = await Promise.all([
    load('day'),
    load('principal'),
    load('model'),
  ]);

  return (
    <AppLayout
      title="Usage"
      workspaceSlug={workspaceSlug}
      environmentId={environmentId}
      shell={shell}
    >
      <UsageContent
        workspaceSlug={shell.activeWorkspace.slug}
        environmentId={shell.activeEnvironment.id}
        period={period}
        dayBuckets={byDay.buckets}
        principalBuckets={byPrincipal.buckets}
        modelBuckets={byModel.buckets}
      />
    </AppLayout>
  );
}

async function safeLoad(workspaceId: string, path: string): Promise<LlmUsageBucketsResponse> {
  try {
    return await rustApiForWorkspace<LlmUsageBucketsResponse>(workspaceId, path, {
      method: 'GET',
    });
  } catch (error) {
    console.error('[usage] failed to load', path, error);
    return { buckets: [] };
  }
}
