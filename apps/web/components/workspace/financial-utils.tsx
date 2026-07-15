import { Badge } from '@/components/ui/badge';
import type {
  AuthorizationEffect,
  FinancialActionOutcome,
  FinancialActionRecord,
  FinancialExecutionStatus,
} from '@trustloopguard/sdk';

type BadgeVariant = 'permit' | 'deny' | 'require_approval' | 'outline' | 'secondary';

const STATUS_VARIANT: Record<FinancialExecutionStatus, BadgeVariant> = {
  not_started: 'outline',
  executing: 'require_approval',
  succeeded: 'permit',
  failed: 'deny',
  canceled: 'secondary',
  reversed: 'secondary',
};

const AUTHORIZATION_VARIANT: Record<AuthorizationEffect, BadgeVariant> = {
  permit: 'permit',
  deny: 'deny',
  transform: 'permit',
  require_approval: 'require_approval',
  defer: 'secondary',
};

const OUTCOME_VARIANT: Record<FinancialActionOutcome['status'], BadgeVariant> = {
  pending: 'outline',
  succeeded: 'permit',
  recovered: 'permit',
  failed: 'deny',
  canceled: 'secondary',
  reversed: 'secondary',
  loss_recorded: 'deny',
  disputed: 'deny',
  recovery_started: 'require_approval',
  unknown: 'outline',
};

export function FinancialStatusBadge({ status }: { status: FinancialExecutionStatus }) {
  return <Badge variant={STATUS_VARIANT[status]}>{titleLabel(status)}</Badge>;
}

export function FinancialAuthorizationBadge({ effect }: { effect: AuthorizationEffect }) {
  return <Badge variant={AUTHORIZATION_VARIANT[effect]}>{titleLabel(effect)}</Badge>;
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
  return date.toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    timeZone: 'America/Los_Angeles',
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

/** Extract a display message from an API error body (JSON `error`/`message` or plain text). */
export function safeError(text: string): string | null {
  try {
    const parsed = JSON.parse(text) as { error?: string; message?: string };
    return parsed.error ?? parsed.message ?? null;
  } catch {
    return text.trim() === '' ? null : text;
  }
}
