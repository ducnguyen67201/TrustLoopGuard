import type { AuthorizationReceipt } from '@featherlane-ai/sdk';
import Link from 'next/link';
import type { ReactNode } from 'react';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { PageHeader } from '@/components/ui/page-header';
import { FinancialAuthorizationBadge, formatDateTime } from './financial-utils';

export function AuthorizationReceiptContent({
  receipt,
  workspaceSlug,
  environmentId,
}: {
  receipt: AuthorizationReceipt;
  workspaceSlug: string;
  environmentId: string;
}) {
  const query = `?workspace=${encodeURIComponent(workspaceSlug)}&environment=${encodeURIComponent(environmentId)}`;
  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <PageHeader
        eyebrow="Immutable authorization proof"
        title="Authorization receipt"
        description="This records what the kernel evaluated. It is audit evidence, not reusable runtime authority."
      />
      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Decision</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 text-sm">
            <div className="flex flex-wrap gap-2">
              <FinancialAuthorizationBadge effect={receipt.effect} />
              <Badge variant="outline">{receipt.domain}</Badge>
              {receipt.intent_status ? (
                <Badge variant="outline">{receipt.intent_status}</Badge>
              ) : null}
            </div>
            <Fact label="Reason" value={receipt.reason} />
            <Fact label="Subject hash" value={receipt.subject_hash} mono />
            <Fact label="Trace" value={receipt.trace_id ?? '—'} mono />
            <Fact label="Intent" value={receipt.intent_id ?? 'Observation only'} mono />
            <Fact label="Principal" value={receipt.principal_id ?? 'Legacy / unknown'} mono />
            <Fact label="Operation" value={receipt.operation ?? 'Legacy / unknown'} mono />
            <Fact
              label="Run"
              value={
                receipt.run_id ? (
                  <Link
                    className="text-primary hover:underline"
                    href={`/runs/${encodeURIComponent(receipt.run_id)}${query}`}
                  >
                    {receipt.run_id}
                  </Link>
                ) : (
                  'Not grouped'
                )
              }
              mono
            />
            <Fact label="Created" value={formatDateTime(receipt.created_at)} />
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Authority and execution</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 text-sm">
            <Fact label="Approval" value={receipt.approval_id ?? 'Not used'} mono />
            <Fact label="Grant" value={receipt.grant_id ?? 'Not used'} mono />
            <Fact label="Lease" value={receipt.lease_id ?? 'Not issued'} mono />
            <Fact
              label="Policy versions"
              value={
                receipt.policy_versions.length > 0 ? receipt.policy_versions.join(', ') : 'None'
              }
              mono
            />
          </CardContent>
        </Card>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Findings</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3">
          {receipt.findings.length === 0 ? (
            <p className="text-sm text-muted-foreground">No enforcing findings.</p>
          ) : (
            receipt.findings.map((finding) => (
              <div key={finding.id} className="grid gap-1 rounded-lg border p-3 text-sm">
                <div className="flex flex-wrap gap-2">
                  <FinancialAuthorizationBadge effect={finding.effect} />
                  <Badge variant="outline">{finding.source}</Badge>
                </div>
                <p>{finding.reason}</p>
                {finding.requirement_id ? (
                  <span className="font-mono text-xs text-muted-foreground">
                    requirement {finding.requirement_id}
                  </span>
                ) : null}
              </div>
            ))
          )}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Domain evidence</CardTitle>
        </CardHeader>
        <CardContent>
          <pre className="overflow-auto rounded-lg bg-muted p-4 text-xs">
            {JSON.stringify(receipt.domain_evidence, null, 2)}
          </pre>
        </CardContent>
      </Card>
    </div>
  );
}

function Fact({ label, value, mono = false }: { label: string; value: ReactNode; mono?: boolean }) {
  return (
    <div className="grid gap-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={mono ? 'break-all font-mono text-xs' : ''}>{value}</span>
    </div>
  );
}
