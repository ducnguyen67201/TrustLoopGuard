import 'server-only';

import type { LlmUsageBucket, LlmUsageBucketsResponse } from '@featherlane-ai/sdk';

import { periodRange, type UsagePeriod } from '@/components/workspace/usage-utils';
import { rustApiForWorkspace } from './tl-client';

export type LlmUsageDashboardData = {
  dayBuckets: LlmUsageBucket[];
  principalBuckets: LlmUsageBucket[];
  modelBuckets: LlmUsageBucket[];
};

export async function getLlmUsageDashboardData(
  workspaceId: string,
  period: UsagePeriod,
  kind: 'customer_inference' | 'guardrail' = 'customer_inference',
): Promise<LlmUsageDashboardData> {
  const { start, end } = periodRange(period, new Date());
  const window = `start=${encodeURIComponent(start.toISOString())}&end=${encodeURIComponent(end.toISOString())}&kind=${kind}`;
  const load = (groupBy: 'day' | 'principal' | 'model') =>
    safeLoad(workspaceId, `/v1/llm-usage?group_by=${groupBy}&${window}`);
  const [byDay, byPrincipal, byModel] = await Promise.all([
    load('day'),
    load('principal'),
    load('model'),
  ]);

  return {
    dayBuckets: byDay.buckets,
    principalBuckets: byPrincipal.buckets,
    modelBuckets: byModel.buckets,
  };
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
