'use client';

import Link from 'next/link';
import { useMemo, useState } from 'react';
import {
  IconBraces,
  IconCheck,
  IconCopy,
  IconDownload,
  IconFileSearch,
  IconRoute,
  IconShieldCheck,
} from '@tabler/icons-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { PageHeader } from '@/components/ui/page-header';
import type { TraceDetailPageData } from '@/lib/server/dashboard-data';

type EvidenceRow = {
  id: string;
  label: string;
  status: string;
  detail: string;
};

type KeyValueRow = {
  id: string;
  label: string;
  value: string;
};

const evidenceColumns: DataTableColumn<EvidenceRow>[] = [
  { id: 'label', header: 'Evidence', cell: (row) => row.label },
  {
    id: 'status',
    header: 'Status',
    cell: (row) => row.status,
    cellClassName: 'font-data text-xs text-muted-foreground',
  },
  {
    id: 'detail',
    header: 'Detail',
    cell: (row) => row.detail,
    cellClassName: 'text-xs text-muted-foreground',
  },
];

const keyValueColumns: DataTableColumn<KeyValueRow>[] = [
  { id: 'label', header: 'Field', cell: (row) => row.label },
  {
    id: 'value',
    header: 'Value',
    cell: (row) => row.value,
    cellClassName: 'font-data text-xs text-muted-foreground break-all',
  },
];

export function TraceDetailPageContent({ data }: { data: TraceDetailPageData }) {
  const trace = data.trace;
  const payload = isRecord(trace.payload) ? trace.payload : {};
  const event = recordAt(payload, 'event');
  const action = recordAt(event, 'action');
  const actionRows = keyValueRows(recordAt(action, 'parameters'));
  const sourceRows = sourceAndProvenanceRows(event, payload);
  const policyRows = policyEvidence(payload);
  const checkRows = checkEvidence(payload);
  const reason = stringAt(payload, 'reason');
  const rawJson = useMemo(() => JSON.stringify(trace, null, 2), [trace]);
  const [copied, setCopied] = useState(false);

  async function copyRawJson() {
    if (!navigator.clipboard) return;
    await navigator.clipboard.writeText(rawJson);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  }

  const runHref =
    trace.run_id && trace.run_id.trim() !== ''
      ? runDetailHref(trace.run_id, data.activeWorkspace.slug, data.activeEnvironment.id)
      : null;

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow={data.activeWorkspace.name}
        title="Trace replay"
        description="A stored proof of what the guardrail saw, which policy evidence mattered, and why the final decision was returned."
        actions={
          <div className="flex flex-wrap gap-2">
            {runHref ? (
              <Button asChild variant="outline">
                <Link href={runHref}>
                  <IconRoute />
                  Open run
                </Link>
              </Button>
            ) : null}
            <Button type="button" variant="outline" onClick={() => void copyRawJson()}>
              {copied ? <IconCheck /> : <IconCopy />}
              Copy raw JSON
            </Button>
            <Button asChild>
              <a
                href={`data:application/json;charset=utf-8,${encodeURIComponent(rawJson)}`}
                download={`${trace.trace_id}.json`}
              >
                <IconDownload />
                Download raw JSON
              </a>
            </Button>
          </div>
        }
      />

      <div className="grid gap-4 lg:grid-cols-4">
        <StatCard label="Verdict" value={verdictLabel(trace.decision)} badge={trace.decision} />
        <StatCard label="Domain" value={trace.domain || 'Unknown'} />
        <StatCard label="Latency" value={`${trace.elapsed_ms}ms`} />
        <StatCard label="Environment" value={trace.environment || trace.environment_id} />
      </div>

      <Card>
        <CardHeader className="border-b pb-5">
          <CardDescription>{trace.trace_id}</CardDescription>
          <CardTitle>Decision summary</CardTitle>
          <CardAction>
            <Badge variant="outline" className="font-data text-xs">
              {formatDate(trace.created_at)}
            </Badge>
          </CardAction>
        </CardHeader>
        <CardContent className="grid gap-4 pt-5">
          <Field label="Reason" value={reason ?? 'No reason recorded'} />
          <div className="grid gap-3 md:grid-cols-3">
            <Field label="Run" value={trace.run_id ?? 'Not linked'} />
            <Field label="Run event" value={trace.run_event_id ?? 'Not linked'} />
            <Field label="Session" value={trace.session_id ?? 'Not tagged'} />
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_minmax(320px,0.65fr)]">
        <Card>
          <CardHeader className="border-b pb-5">
            <CardDescription>{stringAt(action, 'operation') ?? 'No operation'}</CardDescription>
            <CardTitle>Proposed action</CardTitle>
          </CardHeader>
          <CardContent className="pt-5">
            {actionRows.length > 0 ? (
              <DataTable
                columns={keyValueColumns}
                rows={actionRows}
                getRowKey={(row) => row.id}
                caption="Proposed action parameters"
              />
            ) : (
              <EmptyState
                icon={<IconFileSearch />}
                title="No proposed action recorded"
                description="Older traces may only include the final decision payload."
              />
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="border-b pb-5">
            <CardDescription>Source and provenance</CardDescription>
            <CardTitle>Provenance</CardTitle>
          </CardHeader>
          <CardContent className="pt-5">
            {sourceRows.length > 0 ? (
              <DataTable
                columns={keyValueColumns}
                rows={sourceRows}
                getRowKey={(row) => row.id}
                caption="Trace source and provenance"
              />
            ) : (
              <EmptyState
                icon={<IconFileSearch />}
                title="No provenance recorded"
                description="The raw JSON below still contains every stored field."
              />
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader className="border-b pb-5">
          <CardDescription>Policies and checks captured at decision time</CardDescription>
          <CardTitle>Evidence</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-5 pt-5">
          {policyRows.length > 0 ? (
            <DataTable
              columns={evidenceColumns}
              rows={policyRows}
              getRowKey={(row) => row.id}
              caption="Triggered policy evidence"
            />
          ) : (
            <EmptyState
              icon={<IconShieldCheck />}
              title="No policy evidence recorded"
              description="This trace did not include triggered policy details."
            />
          )}
          {checkRows.length > 0 ? (
            <DataTable
              columns={evidenceColumns}
              rows={checkRows}
              getRowKey={(row) => row.id}
              caption="Checker evidence"
            />
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="border-b pb-5">
          <CardDescription>Complete stored payload</CardDescription>
          <CardTitle className="flex items-center gap-2">
            <IconBraces className="size-5 text-primary" />
            Raw JSON
          </CardTitle>
        </CardHeader>
        <CardContent className="pt-5">
          <pre
            aria-label="Raw trace JSON"
            className="max-h-[32rem] overflow-auto rounded-lg border bg-muted/30 p-4 font-mono text-xs leading-relaxed text-foreground"
          >
            {rawJson}
          </pre>
        </CardContent>
      </Card>
    </div>
  );
}

function StatCard({ label, value, badge }: { label: string; value: string; badge?: string }) {
  return (
    <Card>
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle className="truncate text-base" title={value}>
          {badge ? <Badge variant={verdictVariant(badge)}>{value}</Badge> : value}
        </CardTitle>
      </CardHeader>
    </Card>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid gap-1">
      <dt className="text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="break-all font-data text-sm text-foreground">{value}</dd>
    </div>
  );
}

function policyEvidence(payload: Record<string, unknown>): EvidenceRow[] {
  const policies = payload['triggered_policies'];
  if (!Array.isArray(policies)) return [];
  return policies.filter(isRecord).map((policy, index) => {
    const id = stringAt(policy, 'id') ?? `policy-${index + 1}`;
    return {
      id,
      label: id,
      status: stringAt(policy, 'severity') ?? 'triggered',
      detail: stringAt(policy, 'reason') ?? stringAt(policy, 'description') ?? 'Policy triggered',
    };
  });
}

function checkEvidence(payload: Record<string, unknown>): EvidenceRow[] {
  const checks = payload['checks'] ?? payload['checker_runs'] ?? payload['signals'];
  if (!Array.isArray(checks)) return [];
  return checks.filter(isRecord).map((check, index) => {
    const label =
      stringAt(check, 'checker') ??
      stringAt(check, 'name') ??
      stringAt(check, 'id') ??
      `check-${index + 1}`;
    return {
      id: `${label}-${index}`,
      label,
      status: stringAt(check, 'status') ?? stringAt(check, 'verdict') ?? 'recorded',
      detail: stringAt(check, 'message') ?? stringAt(check, 'reason') ?? stringify(check),
    };
  });
}

function keyValueRows(value: Record<string, unknown> | undefined): KeyValueRow[] {
  if (!value) return [];
  return Object.entries(value).map(([key, entry]) => ({
    id: key,
    label: titleize(key),
    value: stringify(entry),
  }));
}

function sourceAndProvenanceRows(
  event: Record<string, unknown> | undefined,
  payload: Record<string, unknown>,
): KeyValueRow[] {
  const rows: KeyValueRow[] = [];
  const sources = event?.['sources'] ?? payload['sources'];
  if (Array.isArray(sources)) {
    sources.forEach((source, index) => {
      if (!isRecord(source)) return;
      const id = stringAt(source, 'id') ?? `source-${index + 1}`;
      rows.push({
        id: `source-${id}`,
        label: `Source ${id}`,
        value: stringify(source),
      });
    });
  }

  const legacySource = recordAt(event, 'source') ?? recordAt(payload, 'source');
  if (rows.length === 0 && legacySource !== undefined) {
    rows.push(...keyValueRows(legacySource));
  }

  const provenance = recordAt(event, 'provenance') ?? recordAt(payload, 'provenance');
  if (provenance !== undefined) {
    Object.entries(provenance).forEach(([field, sourceIds]) => {
      rows.push({
        id: `provenance-${field}`,
        label: `Provenance ${titleize(field)}`,
        value: stringify(sourceIds),
      });
    });
  }

  return rows;
}

function recordAt(value: Record<string, unknown> | undefined, key: string) {
  if (!value) return undefined;
  const next = value[key];
  return isRecord(next) ? next : undefined;
}

function stringAt(value: Record<string, unknown> | undefined, key: string): string | undefined {
  if (!value) return undefined;
  const next = value[key];
  return typeof next === 'string' && next.trim() !== '' ? next : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stringify(value: unknown): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (value === null || value === undefined) return '—';
  return JSON.stringify(value, null, 2);
}

function titleize(value: string): string {
  return value.replace(/[_-]+/g, ' ').replace(/\b\w/g, (char) => char.toUpperCase());
}

function verdictVariant(value: string): 'allow' | 'rewrite' | 'block' | 'escalate' | 'outline' {
  if (value === 'allow' || value === 'rewrite' || value === 'block' || value === 'escalate') {
    return value;
  }
  return 'outline';
}

function verdictLabel(value: string): string {
  return verdictVariant(value) === 'outline' ? value : titleize(value);
}

function formatDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

function runDetailHref(runId: string, workspaceSlug: string, environmentId: string): string {
  const params = new URLSearchParams({ workspace: workspaceSlug, environment: environmentId });
  return `/runs/${encodeURIComponent(runId)}?${params.toString()}`;
}
