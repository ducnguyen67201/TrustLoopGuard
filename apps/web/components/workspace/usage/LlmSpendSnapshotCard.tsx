import { IconArrowRight, IconTrendingUp } from '@tabler/icons-react';
import Link from 'next/link';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';

import { formatMinorUnits } from '../financial-utils';
import { bySpendDescending, spendIntensity, sumBy, USAGE_CURRENCY } from './usage-utils';

const TOP_PRINCIPALS = 3;

type LlmSpendSnapshotCardProps = {
  principalBuckets: LlmUsageBucket[];
  /** Drill-in route for the "View detail" affordance (context-qualified). */
  detailHref: string;
  className?: string;
};

/**
 * The home-dashboard glance: this-week LLM spend plus the top few burners as a
 * mini bar list. Answers "am I fine?" without opening anything — not a shrunk
 * clone of the full page, a purpose-built summary.
 */
export function LlmSpendSnapshotCard({
  principalBuckets,
  detailHref,
  className,
}: LlmSpendSnapshotCardProps) {
  const totalSpendMinor = sumBy(principalBuckets, (bucket) => bucket.cost_minor);
  const ranked = bySpendDescending(principalBuckets);
  const top = ranked.slice(0, TOP_PRINCIPALS);
  const maxCostMinor = Number(top[0]?.cost_minor ?? 0);
  const hasSpend = principalBuckets.length > 0;

  return (
    <Card className={className}>
      <CardHeader>
        <CardDescription>This week&rsquo;s LLM spend</CardDescription>
        <CardTitle className="flex items-center gap-2">
          <IconTrendingUp className="size-5 text-primary" />
          {formatMinorUnits(totalSpendMinor, USAGE_CURRENCY)}
        </CardTitle>
        {hasSpend ? (
          <CardAction>
            <Button asChild size="sm" variant="ghost">
              <Link href={detailHref}>
                View detail
                <IconArrowRight />
              </Link>
            </Button>
          </CardAction>
        ) : null}
      </CardHeader>
      <CardContent className="grid gap-3">
        {hasSpend ? (
          <>
            <p className="text-xs text-muted-foreground">Top spenders this week</p>
            <ul className="grid gap-2.5">
              {top.map((bucket) => (
                <PrincipalBar
                  key={bucket.key}
                  bucket={bucket}
                  maxCostMinor={maxCostMinor}
                />
              ))}
            </ul>
          </>
        ) : (
          <div className="grid gap-1 rounded-lg border border-dashed bg-card/40 px-4 py-6 text-center">
            <p className="text-sm font-medium text-foreground">No spend yet this week</p>
            <p className="mx-auto max-w-xs text-xs text-muted-foreground">
              Point an agent at the gateway and metered spend lands here.
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function PrincipalBar({
  bucket,
  maxCostMinor,
}: {
  bucket: LlmUsageBucket;
  maxCostMinor: number;
}) {
  const intensity = spendIntensity(bucket, maxCostMinor);
  return (
    <li className="grid gap-1">
      <div className="flex items-baseline justify-between gap-3">
        <span className="truncate font-mono text-xs text-foreground" title={bucket.key}>
          {bucket.key}
        </span>
        <span className="font-mono text-xs font-medium tabular-nums text-foreground">
          {formatMinorUnits(bucket.cost_minor, USAGE_CURRENCY)}
        </span>
      </div>
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
        <span
          className="block h-full rounded-full bg-[var(--chart-1)]"
          style={{ width: `${Math.max(4, intensity * 100)}%` }}
          aria-hidden
        />
      </div>
    </li>
  );
}
