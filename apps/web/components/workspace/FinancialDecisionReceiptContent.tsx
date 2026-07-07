import Link from 'next/link';
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
            <p className="text-muted-foreground">
              {receipt.authorization_scope.reason ?? 'No scope detail recorded.'}
            </p>
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

function DecisionBadge({ decision }: { decision: FinancialActionDecisionReceipt['decision'] }) {
  const variant =
    decision === 'allow'
      ? 'allow'
      : decision === 'hold' || decision === 'escalate'
        ? 'escalate'
        : 'block';
  return <Badge variant={variant}>{titleLabel(decision)}</Badge>;
}

function Fact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="grid min-w-0 gap-0.5">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={mono ? 'truncate font-mono text-xs' : 'truncate'}>{value}</span>
    </div>
  );
}
