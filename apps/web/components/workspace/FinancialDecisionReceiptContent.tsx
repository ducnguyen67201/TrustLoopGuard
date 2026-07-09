import Link from 'next/link';
import {
  IconBuildingStore,
  IconChecklist,
  IconFileCertificate,
  IconWallet,
} from '@tabler/icons-react';
import type { FinancialActionDecisionReceipt, FinancialActionRecord } from '@trustloopguard/sdk';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { PageHeader } from '@/components/ui/page-header';
import {
  counterpartyLabel,
  currentContextQuery,
  FinancialStatusBadge,
  formatDateTime,
  formatMinorUnits,
  titleLabel,
} from './financial-utils';

type FinancialDecisionReceiptContentProps = {
  workspaceSlug: string;
  environmentId: string;
  receipt: FinancialActionDecisionReceipt;
  action: FinancialActionRecord | null;
};

export function FinancialDecisionReceiptContent({
  workspaceSlug,
  environmentId,
  receipt,
  action,
}: FinancialDecisionReceiptContentProps) {
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);
  const x402 = action ? x402PaymentContext(action) : null;

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Financial authorization"
        title="Decision receipt"
        description="Per-action proof returned before execution."
        actions={
          <Button asChild variant="outline">
            <Link href={`/financial${contextQuery}`}>Back to ledger</Link>
          </Button>
        }
      />
      <PaymentAuthorizationPath
        receipt={receipt}
        action={action}
        x402={x402}
        contextQuery={contextQuery}
      />
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(20rem,24rem)]">
        <Card>
          <CardHeader>
            <CardTitle>Decision</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            <div className="flex flex-wrap items-center gap-2">
              <DecisionBadge decision={receipt.decision} />
              <FinancialStatusBadge status={receipt.status} />
              <Badge variant="outline">{receipt.operation}</Badge>
            </div>
            <p className="text-sm text-foreground">{receipt.reason}</p>
            <div className="grid gap-2 text-sm md:grid-cols-2">
              <Fact
                label="Amount"
                value={formatMinorUnits(receipt.amount.amount_minor, receipt.amount.currency)}
                mono
              />
              <Fact label="Principal" value={receipt.principal_id} mono />
              <Fact
                label="Counterparty"
                value={
                  action
                    ? counterpartyLabel(action)
                    : (receipt.counterparty?.display_name ?? receipt.counterparty?.id ?? '—')
                }
              />
              <Fact label="Updated" value={formatDateTime(receipt.updated_at)} />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Authorization scope</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 text-sm">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={receipt.authorization_scope.result === 'passed' ? 'allow' : 'block'}>
                {titleLabel(receipt.authorization_scope.result)}
              </Badge>
              <Badge variant="outline">
                {receipt.authorization_scope.checked ? 'Checked' : 'Not checked'}
              </Badge>
            </div>
            <Fact label="Scope" value={receipt.authorization_scope.scope_ref?.id ?? '—'} mono />
            <Fact
              label="Mandate hash"
              value={receipt.authorization_scope.mandate_hash ?? '—'}
              mono
            />
            <p className="text-muted-foreground">
              {receipt.authorization_scope.reason ?? 'No scope detail recorded.'}
            </p>
            {receipt.authorization_scope.normalized_scope ? (
              <pre className="max-h-40 overflow-auto rounded-md border bg-muted/40 p-3 text-xs">
                {JSON.stringify(receipt.authorization_scope.normalized_scope, null, 2)}
              </pre>
            ) : null}
          </CardContent>
        </Card>
      </div>
      <div className="grid gap-4 lg:grid-cols-3">
        <Card>
          <CardHeader>
            <CardTitle>Evidence</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-2">
            {receipt.evidence.length === 0 ? (
              <p className="text-sm text-muted-foreground">No required evidence checks.</p>
            ) : (
              receipt.evidence.map((proof) => (
                <div
                  key={proof.precondition}
                  className="flex min-w-0 items-center justify-between gap-3 rounded-md border px-3 py-2"
                >
                  <span className="truncate text-sm">{titleLabel(proof.precondition)}</span>
                  <Badge variant={proof.status === 'passed' ? 'allow' : 'block'}>
                    {titleLabel(proof.status)}
                  </Badge>
                </div>
              ))
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Risks</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-2">
            {receipt.risks.length === 0 ? (
              <p className="text-sm text-muted-foreground">No risks detected.</p>
            ) : (
              receipt.risks.map((risk) => (
                <div
                  key={`${risk.source}:${risk.code}`}
                  className="grid gap-1 rounded-md border px-3 py-2"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">{titleLabel(risk.code)}</Badge>
                    <Badge variant="secondary">{titleLabel(risk.severity)}</Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">{risk.reason}</p>
                </div>
              ))
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Execution</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 text-sm">
            <Badge variant={receipt.execution.status === 'executed' ? 'allow' : 'outline'}>
              {titleLabel(receipt.execution.status)}
            </Badge>
            <Fact label="Receipt" value={receipt.execution.receipt_id ?? '—'} mono />
            {receipt.execution.ledger_event_ids.length === 0 ? (
              <p className="text-muted-foreground">No ledger event ids recorded.</p>
            ) : (
              receipt.execution.ledger_event_ids.map((id) => (
                <code key={id} className="rounded-md border px-2 py-1 text-xs">
                  {id}
                </code>
              ))
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function PaymentAuthorizationPath({
  receipt,
  action,
  x402,
  contextQuery,
}: {
  receipt: FinancialActionDecisionReceipt;
  action: FinancialActionRecord | null;
  x402: X402PaymentContext | null;
  contextQuery: string;
}) {
  const scope = receipt.authorization_scope.normalized_scope;
  const mandateRef = receipt.authorization_scope.scope_ref;
  const mandateKind = receipt.authorization_scope.scope_snapshot
    ? 'Internal mandate'
    : mandateRef
      ? 'Mandate reference'
      : 'No mandate';
  const mandateLocation = receipt.authorization_scope.scope_snapshot
    ? 'Stored in TrustLoopGuard Financial Mandates'
    : mandateRef
      ? 'Referenced by the action request'
      : 'No mandate was attached to this action';
  const allowedHost = firstString(scope, 'allowed_hosts');
  const allowedResource = firstString(scope, 'allowed_resources');
  const allowedPayTo = firstString(scope, 'allowed_pay_to');
  const maxAmountMinor = numberValue(scope, 'max_amount_minor');
  const maxCurrency = stringValue(scope, 'currency') ?? receipt.amount.currency;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Payment authorization path</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-4">
        <div className="grid gap-3 md:grid-cols-4">
          <PathStep
            icon={<IconFileCertificate />}
            label="Mandate"
            title={mandateKind}
            detail={mandateLocation}
            badge={mandateRef ? `${mandateRef.id} v${mandateRef.version}` : 'missing'}
            badgeTone={mandateRef ? 'allow' : 'escalate'}
          />
          <PathStep
            icon={<IconBuildingStore />}
            label="Commerce"
            title={x402?.host ?? 'Merchant requirement'}
            detail={x402?.resource ?? 'Commerce returns HTTP 402 with payment terms'}
            badge={x402?.hash ? 'x402 402' : 'requirement unavailable'}
            badgeTone={x402?.hash ? 'outline' : 'escalate'}
          />
          <PathStep
            icon={<IconChecklist />}
            label="TrustLoopGuard"
            title={titleLabel(receipt.authorization_scope.result)}
            detail={receipt.authorization_scope.reason ?? 'Mandate and policy checked'}
            badge={receipt.authorization_scope.checked ? 'checked' : 'not checked'}
            badgeTone={receipt.authorization_scope.result === 'passed' ? 'allow' : 'block'}
          />
          <PathStep
            icon={<IconWallet />}
            label="Wallet"
            title={receipt.decision === 'allow' ? 'Allowed to sign' : 'Not signable'}
            detail="The agent can only pay after this authorization result."
            badge={titleLabel(receipt.execution.status)}
            badgeTone={receipt.execution.status === 'executed' ? 'allow' : 'outline'}
          />
        </div>
        <div className="grid gap-3 lg:grid-cols-3">
          <div className="grid gap-2 rounded-md border p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="text-sm font-medium">Mandate boundary</p>
              <Button asChild variant="outline" size="sm">
                <Link href={`/financial/mandates${contextQuery}`}>Show mandates</Link>
              </Button>
            </div>
            <BoundaryFact label="Max" value={formatOptionalMoney(maxAmountMinor, maxCurrency)} />
            <BoundaryFact label="Host" value={allowedHost ?? '—'} mono />
            <BoundaryFact label="Resource" value={allowedResource ?? '—'} mono />
            <BoundaryFact label="Pay to" value={allowedPayTo ?? '—'} mono />
            <BoundaryFact
              label="Mandate hash"
              value={receipt.authorization_scope.mandate_hash ?? '—'}
              mono
            />
          </div>
          <div className="grid gap-2 rounded-md border p-3">
            <p className="text-sm font-medium">Commerce returned</p>
            <BoundaryFact
              label="Amount"
              value={formatMinorUnits(receipt.amount.amount_minor, receipt.amount.currency)}
            />
            <BoundaryFact label="Host" value={x402?.host ?? '—'} mono />
            <BoundaryFact label="Resource" value={x402?.resource ?? '—'} mono />
            <BoundaryFact
              label="Network / asset"
              value={joinPresent([x402?.network, x402?.asset])}
            />
            <BoundaryFact label="Requirement hash" value={x402?.hash ?? '—'} mono />
          </div>
          <div className="grid gap-2 rounded-md border p-3">
            <p className="text-sm font-medium">Why approved</p>
            <CheckFact
              label="Mandate active"
              passed={receipt.authorization_scope.result === 'passed'}
            />
            <CheckFact
              label="Amount inside mandate"
              passed={
                maxAmountMinor === null || Number(receipt.amount.amount_minor) <= maxAmountMinor
              }
            />
            <CheckFact
              label="Merchant/resource matched"
              passed={
                matchesString(scope, 'allowed_hosts', x402?.host) &&
                matchesString(scope, 'allowed_resources', x402?.resource)
              }
            />
            <CheckFact
              label="Pay-to/network/asset matched"
              passed={
                matchesString(scope, 'allowed_pay_to', x402?.payTo) &&
                matchesString(scope, 'allowed_networks', x402?.network) &&
                matchesString(scope, 'allowed_assets', x402?.asset)
              }
            />
          </div>
        </div>
        {action?.action.mandate ? (
          <p className="text-sm text-muted-foreground">
            In this demo, the mandate is internal: TrustLoopGuard created and stored it before the
            agent retried the merchant request. The commerce server stores only its payment
            requirement; it does not store or approve the mandate.
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}

function PathStep({
  icon,
  label,
  title,
  detail,
  badge,
  badgeTone,
}: {
  icon: React.ReactNode;
  label: string;
  title: string;
  detail: string;
  badge: string;
  badgeTone: React.ComponentProps<typeof Badge>['variant'];
}) {
  return (
    <div className="grid gap-3 rounded-md border p-3">
      <div className="flex items-center justify-between gap-3">
        <span className="text-muted-foreground [&>svg]:size-4">{icon}</span>
        <Badge variant={badgeTone}>{badge}</Badge>
      </div>
      <div className="grid min-w-0 gap-1">
        <span className="text-xs uppercase text-muted-foreground">{label}</span>
        <span className="truncate text-sm font-medium">{title}</span>
        <span className="line-clamp-2 text-xs text-muted-foreground">{detail}</span>
      </div>
    </div>
  );
}

function BoundaryFact({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="grid min-w-0 gap-0.5">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={mono ? 'truncate font-mono text-xs' : 'truncate text-sm'}>{value}</span>
    </div>
  );
}

function CheckFact({ label, passed }: { label: string; passed: boolean }) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border px-3 py-2">
      <span className="text-sm">{label}</span>
      <Badge variant={passed ? 'allow' : 'escalate'}>{passed ? 'Passed' : 'Not available'}</Badge>
    </div>
  );
}

function DecisionBadge({ decision }: { decision: FinancialActionDecisionReceipt['decision'] }) {
  const variant =
    decision === 'allow'
      ? 'allow'
      : decision === 'hold' || decision === 'escalate'
        ? 'escalate'
        : 'block';
  return <Badge variant={variant}>{titleLabel(decision)}</Badge>;
}

type JsonRecord = Record<string, unknown>;

type X402PaymentContext = {
  host: string | null;
  resource: string | null;
  network: string | null;
  asset: string | null;
  payTo: string | null;
  hash: string | null;
};

function x402PaymentContext(action: FinancialActionRecord): X402PaymentContext | null {
  const metadata = action.action.metadata;
  if (!isJsonRecord(metadata)) return null;
  const x402 = recordValue(metadata, 'x402');
  const normalized = recordValue(x402, 'normalized_requirement');
  const requirement = recordValue(x402, 'payment_requirement');
  return {
    host: stringValue(normalized, 'host') ?? stringValue(requirement, 'host'),
    resource: stringValue(normalized, 'resource') ?? stringValue(requirement, 'resource'),
    network: stringValue(normalized, 'network') ?? stringValue(requirement, 'network'),
    asset: stringValue(normalized, 'asset') ?? stringValue(requirement, 'asset'),
    payTo: stringValue(normalized, 'pay_to') ?? stringValue(requirement, 'pay_to'),
    hash: stringValue(normalized, 'payment_requirement_hash'),
  };
}

function recordValue(source: unknown, key: string): JsonRecord | null {
  if (!isJsonRecord(source)) return null;
  const value = source[key];
  return isJsonRecord(value) ? value : null;
}

function stringValue(source: unknown, key: string): string | null {
  if (!isJsonRecord(source)) return null;
  const value = source[key];
  return typeof value === 'string' && value.trim() !== '' ? value : null;
}

function numberValue(source: unknown, key: string): number | null {
  if (!isJsonRecord(source)) return null;
  const value = source[key];
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function firstString(source: unknown, key: string): string | null {
  if (!isJsonRecord(source)) return null;
  const value = source[key];
  if (typeof value === 'string' && value.trim() !== '') return value;
  if (!Array.isArray(value)) return null;
  const first = value.find((item) => typeof item === 'string' && item.trim() !== '');
  return typeof first === 'string' ? first : null;
}

function matchesString(
  source: unknown,
  key: string,
  candidate: string | null | undefined,
): boolean {
  if (!candidate || !isJsonRecord(source)) return false;
  const value = source[key];
  if (typeof value === 'string') return value === candidate;
  if (!Array.isArray(value)) return false;
  return value.some((item) => item === candidate);
}

function isJsonRecord(value: unknown): value is JsonRecord {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function formatOptionalMoney(amountMinor: number | null, currency: string): string {
  if (amountMinor === null) return '—';
  return formatMinorUnits(amountMinor, currency);
}

function joinPresent(values: Array<string | null | undefined>): string {
  const present = values.filter((value): value is string => Boolean(value));
  return present.length === 0 ? '—' : present.join(' / ');
}

function Fact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="grid min-w-0 gap-0.5">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={mono ? 'truncate font-mono text-xs' : 'truncate'}>{value}</span>
    </div>
  );
}
