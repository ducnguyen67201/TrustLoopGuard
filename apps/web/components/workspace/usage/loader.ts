import 'server-only';

import { rustApiForWorkspace } from '@/lib/server/tl-client';
import type { LlmUsageBucket, LlmUsageBucketsResponse } from '@trustloopguard/sdk';

import { periodRange, type UsagePeriod } from './usage-utils';

/**
 * The pre-fetched bucket data every usage widget consumes. Whatever a surface
 * embeds — the full drill-in, the Financial section, or the home snapshot — it
 * gets exactly this shape from one loader call, so the widgets never reach for
 * page-level context.
 */
export type UsageBuckets = {
  period: UsagePeriod;
  /** `?principal=` focus, when the surface is drilling into one principal. */
  principalId: string | null;
  day: LlmUsageBucket[];
  principal: LlmUsageBucket[];
  model: LlmUsageBucket[];
};

type LoadOptions = {
  period: UsagePeriod;
  /** Focus a single principal: filters day + model buckets to their spend. */
  principalId?: string | null;
  /** Skip the by-day series when a surface only needs the principal list. */
  groups?: readonly ('day' | 'principal' | 'model')[];
  now?: Date;
};

const ALL_GROUPS = ['day', 'principal', 'model'] as const;

/**
 * One call, embeddable anywhere: resolves the period window, fetches the
 * requested `group_by` rollups for a workspace, and returns the {@link UsageBuckets}
 * the widgets expect. Mirrors the mandates-page server-loader pattern — failures
 * degrade to empty buckets rather than throwing the whole page.
 */
export async function loadUsageBuckets(
  workspaceId: string,
  { period, principalId = null, groups = ALL_GROUPS, now = new Date() }: LoadOptions,
): Promise<UsageBuckets> {
  const { start, end } = periodRange(period, now);
  const params = new URLSearchParams();
  params.set('start', start.toISOString());
  params.set('end', end.toISOString());
  if (principalId) params.set('principal_id', principalId);

  const wanted = new Set(groups);
  const load = (groupBy: 'day' | 'principal' | 'model') =>
    wanted.has(groupBy)
      ? fetchBuckets(workspaceId, groupBy, params)
      : Promise.resolve<LlmUsageBucket[]>([]);

  const [day, principal, model] = await Promise.all([
    load('day'),
    load('principal'),
    load('model'),
  ]);

  return { period, principalId, day, principal, model };
}

async function fetchBuckets(
  workspaceId: string,
  groupBy: 'day' | 'principal' | 'model',
  window: URLSearchParams,
): Promise<LlmUsageBucket[]> {
  const query = new URLSearchParams(window);
  query.set('group_by', groupBy);
  try {
    const response = await rustApiForWorkspace<LlmUsageBucketsResponse>(
      workspaceId,
      `/v1/llm-usage?${query.toString()}`,
      { method: 'GET' },
    );
    return response.buckets;
  } catch (error) {
    console.error('[usage] failed to load', groupBy, error);
    return [];
  }
}
