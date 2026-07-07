import { IconArrowLeft } from '@tabler/icons-react';
import Link from 'next/link';

import { Button } from '@/components/ui/button';
import { EmptyState } from '@/components/ui/empty-state';
import { PageHeader } from '@/components/ui/page-header';

import { currentContextQuery } from './financial-utils';
import {
  SpendByModelCard,
  SpendByPrincipalCard,
  SpendOverTimeCard,
  UsagePeriodSelector,
  UsageSummaryTiles,
  type UsageBuckets,
} from './usage';

type UsageContentProps = {
  workspaceSlug: string;
  environmentId: string;
  buckets: UsageBuckets;
};

/**
 * The `/usage` drill-in: a thin composition of the widget library, reached by
 * drilling from an embedded card — not a sidebar tab. Renders the full board,
 * or a single principal's spend when `?principal=` focuses one.
 */
export function UsageContent({ workspaceSlug, environmentId, buckets }: UsageContentProps) {
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);
  const { period, principalId, day, principal, model } = buckets;
  const focused = principalId !== null;
  const hasUsage = day.length > 0 || principal.length > 0 || model.length > 0;

  // Period links must survive the focus: keep `?principal=` on the base href so
  // switching day/week/month doesn't drop the drill-in.
  const periodHrefBase = focused
    ? `/usage${contextQuery}&principal=${encodeURIComponent(principalId)}`
    : `/usage${contextQuery}`;

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow={focused ? 'Usage · Principal' : 'Monitor'}
        title={focused ? principalId : 'Usage'}
        description={
          focused
            ? 'Spend over time and per-model breakdown for this principal, metered at the LLM gateway.'
            : 'Token consumption and spend per principal and model, metered at the LLM gateway.'
        }
        actions={<UsagePeriodSelector period={period} hrefBase={periodHrefBase} />}
      />

      {focused ? (
        <Button asChild variant="ghost" size="sm" className="w-fit -mt-1 text-muted-foreground">
          <Link href={`/usage${contextQuery}&period=${period}`}>
            <IconArrowLeft />
            All principals
          </Link>
        </Button>
      ) : null}

      {!hasUsage ? (
        <EmptyState
          title="No usage yet"
          description="Point an agent at the gateway — metered LLM calls will show up here."
        />
      ) : focused ? (
        <>
          <UsageSummaryTiles principalBuckets={principal} hidePrincipalCount />
          <SpendOverTimeCard dayBuckets={day} period={period} />
          <SpendByModelCard modelBuckets={model} />
        </>
      ) : (
        <>
          <UsageSummaryTiles principalBuckets={principal} />
          <SpendOverTimeCard dayBuckets={day} period={period} />
          <SpendByPrincipalCard principalBuckets={principal} drillHrefBase={periodHrefBase} />
          <SpendByModelCard modelBuckets={model} />
        </>
      )}
    </div>
  );
}
