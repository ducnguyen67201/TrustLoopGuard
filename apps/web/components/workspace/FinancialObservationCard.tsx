import type {
  FinancialObservationSummaryResponse,
  FinancialRuntimeMode,
} from '@trustloopguard/sdk';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { formatMinorUnits, titleLabel } from './financial-utils';

type FinancialObservationCardProps = {
  mode: FinancialRuntimeMode;
  summary: FinancialObservationSummaryResponse;
};

export function FinancialObservationCard({ mode, summary }: FinancialObservationCardProps) {
  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between gap-3">
        <div className="grid gap-1">
          <CardTitle>Financial pilot</CardTitle>
          <p className="text-sm text-muted-foreground">
            Counterfactual exposure and approval burden for this environment.
          </p>
        </div>
        <Badge variant={mode === 'observe' ? 'secondary' : 'outline'}>
          {mode === 'observe' ? 'Observe' : 'Enforce'}
        </Badge>
      </CardHeader>
      <CardContent className="grid gap-4">
        {summary.currencies.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No observed financial actions in this reporting window.
          </p>
        ) : (
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {summary.currencies.map((currency) => (
              <div key={currency.currency} className="grid gap-2 rounded-lg border p-4">
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium">{currency.currency} exposure</span>
                  <span className="font-mono text-sm tabular-nums">
                    {formatMinorUnits(currency.total_observed_amount_minor, currency.currency)}
                  </span>
                </div>
                <p className="text-sm text-muted-foreground">
                  {currency.total_observed_count} observed · {currency.would_allow_count} would allow
                  · {currency.would_hold_count} would hold · {currency.would_block_count} would block
                </p>
                <p className="text-sm">
                  Approval burden: {currency.estimated_approval_count} (
                  {(currency.estimated_approval_rate_bps / 100).toFixed(1)}%)
                </p>
                <p className="text-sm">
                  Reviewed false positives: {currency.false_positive_count} /{' '}
                  {currency.reviewed_adverse_count} (
                  {(currency.false_positive_rate_bps / 100).toFixed(1)}%)
                </p>
              </div>
            ))}
          </div>
        )}
        {summary.reasons.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {summary.reasons.slice(0, 5).map((reason) => (
              <Badge key={`${reason.outcome}:${reason.amount.currency}:${reason.reason}`} variant="outline">
                {titleLabel(reason.outcome)} · {reason.count}: {reason.reason}
              </Badge>
            ))}
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}
