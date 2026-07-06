import Link from 'next/link';
import type {
  FinancialActionOutcome,
  FinancialActionRecord,
  FinancialReceipt,
} from '@trustloopguard/sdk';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { PageHeader } from '@/components/ui/page-header';
import {
  counterpartyLabel,
  currentContextQuery,
  FinancialStatusBadge,
  formatDateTime,
  formatMoney,
  OutcomeBadge,
  titleLabel,
} from './financial-utils';

type FinancialReceiptContentProps = {
  workspaceSlug: string;
  environmentId: string;
  receipt: FinancialReceipt;
  action: FinancialActionRecord | null;
  outcomes: FinancialActionOutcome[];
};

export function FinancialReceiptContent({
  workspaceSlug,
  environmentId,
  receipt,
  action,
  outcomes,
}: FinancialReceiptContentProps) {
  const contextQuery = currentContextQuery(workspaceSlug, environmentId);
  const latest = outcomes[0];
  const proof = isRecord(receipt.proof) ? receipt.proof : {};
  const providerValue = proof['provider'];
  const provider = isRecord(providerValue) ? providerValue : null;

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Financial authorization"
        title="Receipt"
        description="Ledger-backed proof for an executed financial action."
        actions={
          <Button asChild variant="outline">
            <Link href={`/financial${contextQuery}`}>Back to ledger</Link>
          </Button>
        }
      />
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(20rem,24rem)]">
        <Card>
          <CardHeader>
            <CardTitle>Action</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            {action ? (
              <div className="grid gap-3">
                <div className="flex flex-wrap items-center gap-2">
                  <FinancialStatusBadge status={action.status} />
                  <Badge variant="outline">{titleLabel(action.action.kind)}</Badge>
                  <Badge variant="outline">{titleLabel(action.action.rail)}</Badge>
                </div>
                <div className="grid gap-2 text-sm md:grid-cols-2">
                  <Fact label="Amount" value={formatMoney(action)} mono />
                  <Fact label="Principal" value={action.action.principal_id} mono />
                  <Fact label="Counterparty" value={counterpartyLabel(action)} />
                  <Fact label="Created" value={formatDateTime(action.created_at)} />
                  <Fact label="Mandate" value={action.action.mandate?.id ?? '—'} mono />
                  <Fact label="Trace" value={receipt.trace_id ?? '—'} mono />
                </div>
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">Action detail is unavailable.</p>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Outcome</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3">
            <OutcomeBadge outcome={latest} />
            {latest ? (
              <div className="grid gap-2 text-sm">
                <Fact label="Provider status" value={latest.provider_status ?? '—'} />
                <Fact label="Provider reference" value={latest.provider_reference ?? '—'} mono />
                <Fact label="Reversal" value={titleLabel(latest.reversal_capability)} />
                <Fact label="Observed" value={formatDateTime(latest.occurred_at)} />
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">No provider outcome recorded.</p>
            )}
          </CardContent>
        </Card>
      </div>
      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Provider proof</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-2 text-sm">
            <Fact label="Status" value={stringValue(provider?.['status'])} />
            <Fact label="Reference" value={stringValue(provider?.['reference'])} mono />
            <Fact label="Recovery" value={stringValue(provider?.['recovery_status'])} />
            <pre className="max-h-72 overflow-auto rounded-md border bg-muted p-3 text-xs">
              {JSON.stringify(provider?.['response'] ?? null, null, 2)}
            </pre>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Ledger evidence</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-2">
            {receipt.ledger_event_ids.length === 0 ? (
              <p className="text-sm text-muted-foreground">No ledger event ids recorded.</p>
            ) : (
              receipt.ledger_event_ids.map((id) => (
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

function Fact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="grid min-w-0 gap-0.5">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={mono ? 'truncate font-mono text-xs' : 'truncate'}>{value}</span>
    </div>
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stringValue(value: unknown): string {
  return typeof value === 'string' && value.trim() !== '' ? value : '—';
}
