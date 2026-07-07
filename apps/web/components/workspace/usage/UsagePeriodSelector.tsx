import Link from 'next/link';

import { Button } from '@/components/ui/button';

import { titleLabel } from '../financial-utils';
import { USAGE_PERIODS, type UsagePeriod } from './usage-utils';

type UsagePeriodSelectorProps = {
  period: UsagePeriod;
  /**
   * Base href the period links point at, already carrying workspace/environment
   * (and any `?principal=`) context. `&period=` is appended per option.
   */
  hrefBase: string;
};

/** Segmented day/week/month control. Links, not state — server re-fetches per period. */
export function UsagePeriodSelector({ period, hrefBase }: UsagePeriodSelectorProps) {
  const separator = hrefBase.includes('?') ? '&' : '?';
  return (
    <div
      className="flex items-center gap-1 rounded-lg border bg-card p-1"
      role="group"
      aria-label="Usage period"
    >
      {USAGE_PERIODS.map((option) => (
        <Button key={option} asChild size="sm" variant={option === period ? 'secondary' : 'ghost'}>
          <Link
            href={`${hrefBase}${separator}period=${option}`}
            aria-current={option === period ? 'page' : undefined}
          >
            {titleLabel(option)}
          </Link>
        </Button>
      ))}
    </div>
  );
}
