import { Badge } from '@/components/ui/badge';
import type {
  FinancialActionOutcome,
  FinancialActionRecord,
  FinancialActionStatus,
  FinancialMandate,
} from '@trustloopguard/sdk';

type BadgeVariant = 'allow' | 'block' | 'escalate' | 'outline' | 'secondary';

const STATUS_VARIANT: Record<FinancialActionStatus, BadgeVariant> = {
  proposed: 'outline',
  authorized: 'outline',
  held: 'escalate',
  executed: 'allow',
  denied: 'block',
  failed: 'block',
  reversed: 'secondary',
  expired: 'secondary',
};

const OUTCOME_VARIANT: Record<FinancialActionOutcome['status'], BadgeVariant> = {
  pending: 'outline',
  succeeded: 'allow',
  recovered: 'allow',
  failed: 'block',
  canceled: 'secondary',
  reversed: 'secondary',
  loss_recorded: 'block',
  disputed: 'block',
  recovery_started: 'escalate',
  unknown: 'outline',
};

export function FinancialStatusBadge({ status }: { status: FinancialActionStatus }) {
  return <Badge variant={STATUS_VARIANT[status]}>{titleLabel(status)}</Badge>;
}

export function OutcomeBadge({ outcome }: { outcome: FinancialActionOutcome | undefined }) {
  if (!outcome) {
    return <Badge variant="outline">No outcome</Badge>;
  }
  const variant = OUTCOME_VARIANT[outcome.status];
  return (
    <span className="inline-flex flex-wrap items-center gap-1.5">
      <Badge variant={variant}>{titleLabel(outcome.status)}</Badge>
      <Badge variant="outline">{titleLabel(outcome.recovery_status)}</Badge>
    </span>
  );
}

export function MandateStatusBadge({ mandate }: { mandate: FinancialMandate }) {
  const variant: BadgeVariant =
    mandate.status === 'active' ? 'allow' : mandate.status === 'revoked' ? 'secondary' : 'escalate';
  return <Badge variant={variant}>{titleLabel(mandate.status)}</Badge>;
}

export function formatMoney(action: FinancialActionRecord): string {
  return formatMinorUnits(action.action.amount.amount_minor, action.action.amount.currency);
}

// Locale is pinned to en-US so server render, client hydration, and tests all
// agree on the output.
export function formatMinorUnits(amountMinor: number | bigint, currency: string): string {
  if (typeof amountMinor === 'bigint') {
    const sign = amountMinor < 0n ? '-' : '';
    const absolute = amountMinor < 0n ? -amountMinor : amountMinor;
    const major = absolute / 100n;
    const minor = absolute % 100n;
    const majorText = new Intl.NumberFormat('en-US', { useGrouping: true }).format(major);
    const minorText = minor.toString().padStart(2, '0');
    const formatter = new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
    let insertedInteger = false;
    return (
      sign +
      formatter
        .formatToParts(0)
        .map((part) => {
          if (part.type === 'integer') {
            if (insertedInteger) return '';
            insertedInteger = true;
            return majorText;
          }
          if (part.type === 'fraction') return minorText;
          return part.value;
        })
        .join('')
    );
  }
  return (Number(amountMinor) / 100).toLocaleString('en-US', {
    style: 'currency',
    currency,
  });
}

export function formatDateTime(value?: string | null): string {
  if (!value) return '—';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
}

export function titleLabel(value: string): string {
  return value
    .split('_')
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
    .join(' ');
}

export function latestOutcome(
  outcomes: Record<string, FinancialActionOutcome[]>,
  actionId: string,
): FinancialActionOutcome | undefined {
  return outcomes[actionId]?.[0];
}

export function currentContextQuery(workspaceSlug: string, environmentId: string): string {
  const params = new URLSearchParams();
  params.set('workspace', workspaceSlug);
  params.set('environment', environmentId);
  return `?${params.toString()}`;
}

export function counterpartyLabel(action: FinancialActionRecord): string {
  const counterparty = action.action.counterparty;
  return counterparty?.display_name ?? counterparty?.id ?? '—';
}
