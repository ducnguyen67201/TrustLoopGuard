import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { formatMinorUnits } from '../financial-utils';
import { formatTokens, sumBy, USAGE_CURRENCY } from './usage-utils';

type UsageSummaryTilesProps = {
  /** Per-principal buckets — the ground truth for spend, tokens, and headcount. */
  principalBuckets: LlmUsageBucket[];
  /** Drop the principal-count tile when the surface is already focused on one. */
  hidePrincipalCount?: boolean;
  className?: string;
};

/**
 * Total spend / tokens / active-principals, rendered as one glanceable row.
 * Pure presentation over pre-fetched buckets — droppable into any grid.
 */
export function UsageSummaryTiles({
  principalBuckets,
  hidePrincipalCount = false,
  className,
}: UsageSummaryTilesProps) {
  const totalSpendMinor = sumBy(principalBuckets, (bucket) => bucket.cost_minor);
  const tokens =
    sumBy(principalBuckets, (bucket) => bucket.prompt_tokens) +
    sumBy(principalBuckets, (bucket) => bucket.completion_tokens);

  const columns = hidePrincipalCount ? 'sm:grid-cols-2' : 'sm:grid-cols-3';

  return (
    <div className={`grid gap-3 ${columns} ${className ?? ''}`.trim()}>
      <Tile label="Total spend" value={formatMinorUnits(totalSpendMinor, USAGE_CURRENCY)} accent />
      <Tile label="Total tokens" value={formatTokens(tokens)} />
      {hidePrincipalCount ? null : (
        <Tile
          label="Active principals"
          value={principalBuckets.length.toLocaleString('en-US')}
        />
      )}
    </div>
  );
}

function Tile({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="rounded-lg border bg-card px-4 py-3">
      <p className="text-xs uppercase text-muted-foreground">{label}</p>
      <p
        className={`mt-1 font-mono text-2xl font-semibold tabular-nums ${
          accent ? 'text-foreground' : 'text-foreground'
        }`}
      >
        {value}
      </p>
    </div>
  );
}
