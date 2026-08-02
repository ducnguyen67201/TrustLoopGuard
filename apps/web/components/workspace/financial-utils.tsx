import { Badge } from '@/components/ui/badge';
import type {
  AuthorizationEffect,
  FinancialActionOutcome,
  FinancialActionRecord,
  FinancialActionState,
  FinancialExecutionStatus,
} from '@featherlane-ai/sdk';

type BadgeVariant = 'permit' | 'deny' | 'require_approval' | 'outline' | 'secondary';
type FinancialActionWithOptionalState = Omit<FinancialActionRecord, 'state'> & {
  state?: FinancialActionState;
};

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

const ACTION_STATE_VARIANT: Record<FinancialActionState, BadgeVariant> = {
  evaluating: 'secondary',
  authorized: 'permit',
  held_for_approval: 'require_approval',
  blocked: 'deny',
  not_executable: 'deny',
  executing: 'require_approval',
  executed: 'permit',
  failed: 'deny',
  canceled: 'secondary',
  reversed: 'secondary',
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

export function FinancialActionStateBadge({ state }: { state: FinancialActionState }) {
  return <Badge variant={ACTION_STATE_VARIANT[state]}>{titleLabel(state)}</Badge>;
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

export function effectiveFinancialActionState(
  action: FinancialActionWithOptionalState,
): FinancialActionState {
  if (action.state) return action.state;
  if (action.execution_status === 'succeeded') return 'executed';
  if (action.execution_status === 'failed') return 'failed';
  if (action.execution_status === 'executing') return 'executing';
  if (action.execution_status === 'canceled') return 'canceled';
  if (action.execution_status === 'reversed') return 'reversed';
  if (!action.authorization_intent_id && firstFailedFinancialEvidenceReason(action)) {
    return 'not_executable';
  }
  if (action.authorization_effect === 'deny') return 'blocked';
  if (action.authorization_effect === 'require_approval') return 'held_for_approval';
  if (action.authorization_effect === 'permit' || action.authorization_effect === 'transform') {
    return 'authorized';
  }
  return 'evaluating';
}

const EVIDENCE_FAILURE_LABELS = [
  ['order_exists', 'Order not found'],
  ['payment_captured', 'Payment was not captured'],
  ['refund_window_open', 'Refund window closed'],
  ['amount_lte_refundable_balance', 'Amount exceeds refundable balance'],
  ['destination_is_original_payment_method', 'Not original payment method'],
  ['no_duplicate_refund', 'Duplicate refund'],
  ['invoice_matches_po', 'Invoice does not match PO'],
  ['vendor_approved', 'Vendor not approved'],
  ['grant_valid', 'Grant invalid'],
] as const;

export function firstFailedFinancialEvidenceReason(
  action: Pick<FinancialActionRecord, 'evidence'>,
): string | null {
  for (const evidence of action.evidence) {
    const metadata = evidence.metadata;
    if (!isRecord(metadata)) continue;
    for (const [key, label] of EVIDENCE_FAILURE_LABELS) {
      if (metadata[key] === false) return label;
    }
  }
  return null;
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
