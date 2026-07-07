import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { spendIntensity } from './usage-utils';

/**
 * The one heat treatment shared by every principal list — the hero table, the
 * Financial embed, and the home snapshot — so all three read as one family: a
 * solid `var(--chart-1)` fill on a muted track, scaled to the heaviest spender
 * in its set. A minimum sliver keeps a tiny-but-nonzero spender visible.
 */
export function SpendIntensityBar({
  bucket,
  maxCostMinor,
  className,
}: {
  bucket: LlmUsageBucket;
  maxCostMinor: number;
  className?: string;
}) {
  const intensity = spendIntensity(bucket, maxCostMinor);
  return (
    <div className={`h-1.5 w-full overflow-hidden rounded-full bg-muted ${className ?? ''}`.trim()}>
      <span
        aria-hidden
        className="block h-full rounded-full bg-[var(--chart-1)]"
        style={{ width: `${Math.max(4, intensity * 100)}%` }}
      />
    </div>
  );
}
