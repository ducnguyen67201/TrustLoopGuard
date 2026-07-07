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
  // The money tile leads the row: a primary-tinted ring + label and a heavier,
  // primary-colored numeral so spend wins the first fixation.
  return (
    <div
      className={`rounded-lg border px-4 py-3 ${
        accent ? 'border-primary/30 bg-primary/[0.04] ring-1 ring-primary/10' : 'bg-card'
      }`}
    >
      <p className={`text-xs uppercase ${accent ? 'text-primary' : 'text-muted-foreground'}`}>
        {label}
      </p>
      <p
        className={`mt-1 font-mono text-2xl tabular-nums ${
          accent ? 'font-bold text-primary' : 'font-semibold text-foreground'
        }`}
      >
        {value}
      </p>
    </div>
  );
}
