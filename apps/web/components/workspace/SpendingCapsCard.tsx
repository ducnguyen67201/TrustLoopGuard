import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { FamilyPolicyRow } from '@/lib/server/dashboard-data';

/**
 * Read-only listing of payment-family spend caps. Caps are created through
 * the pay MCP (`set_policy`) and enforced inline by the pay gate; this card
 * makes them visible next to the content rules so an operator can confirm a
 * cap exists without calling the API.
 */
export function SpendingCapsCard({ policies }: { policies: FamilyPolicyRow[] }) {
  const caps = policies.filter((policy) => policy.family === 'payment');
  if (caps.length === 0) return null;

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between gap-3 space-y-0">
        <CardTitle>Spending caps</CardTitle>
        <span className="text-xs text-muted-foreground">
          Set via the pay MCP · enforced on every spend
        </span>
      </CardHeader>
      <CardContent>
        <div className="grid gap-2">
          {caps.map((cap) => (
            <div
              key={cap.id}
              className="flex flex-wrap items-center justify-between gap-2 rounded-md border px-3 py-2"
            >
              <div className="flex items-center gap-2">
                <span className="font-medium">{ownerLabel(cap)}</span>
                <span className="text-xs text-muted-foreground">{cap.id}</span>
              </div>
              <div className="flex flex-wrap items-center gap-1.5">
                <CapBadge label="per transaction" minor={cap.per_transaction_minor} />
                <CapBadge label="daily" minor={cap.daily_minor} />
                <CapBadge label="monthly" minor={cap.monthly_minor} />
                <CapBadge label="hold above" minor={cap.hold_above_minor} />
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function ownerLabel(cap: FamilyPolicyRow): string {
  const agents = cap.when?.agents ?? [];
  return agents.length > 0 ? agents.join(', ') : 'All owners';
}

function CapBadge({ label, minor }: { label: string; minor: number | null | undefined }) {
  if (minor == null) return null;
  return (
    <Badge variant="outline" className="tabular-nums font-normal">
      {label} {formatMinorUnits(minor)}
    </Badge>
  );
}

/** Amounts are stored as minor units (cents); display as currency. */
function formatMinorUnits(minor: number): string {
  return (minor / 100).toLocaleString(undefined, {
    style: 'currency',
    currency: 'USD',
  });
}
