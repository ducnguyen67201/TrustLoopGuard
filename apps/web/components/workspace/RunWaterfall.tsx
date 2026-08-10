'use client';

import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import { Check, ChevronDown, ChevronRight, Copy } from 'lucide-react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { RunDetailSnapshot } from '@/lib/run-detail-live';

type RunSpan = RunDetailSnapshot['spans'][number];

type SpanNode = {
  span: RunSpan;
  parentKey: string | null;
  children: SpanNode[];
};

type VisibleSpan = {
  span: RunSpan;
  parentKey: string | null;
  depth: number;
  childCount: number;
};

const SERVICE_TONES = [
  'bg-chart-1',
  'bg-chart-2',
  'bg-chart-3',
  'bg-chart-4',
  'bg-chart-5',
] as const;

// Reimplements the MIT-licensed otel-gui interaction model for this React surface;
// span data continues to come from the Rust-owned RunDetail contract.
export function RunWaterfall({ spans }: { spans: RunDetailSnapshot['spans'] }) {
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());
  const [selectedKey, setSelectedKey] = useState<string | null>(() => spans[0]?.key ?? null);
  const rowRefs = useRef(new Map<string, HTMLButtonElement>());
  const roots = useMemo(() => buildSpanTree(spans), [spans]);
  const visible = useMemo(() => visibleSpans(roots, collapsed), [collapsed, roots]);
  const selected = spans.find((span) => span.key === selectedKey) ?? visible[0]?.span ?? null;
  const domain = useMemo(() => spanDomain(spans), [spans]);
  const serviceTones = useMemo(() => serviceToneMap(spans), [spans]);

  useEffect(() => {
    if (selectedKey !== null && spans.some((span) => span.key === selectedKey)) return;
    setSelectedKey(spans[0]?.key ?? null);
  }, [selectedKey, spans]);

  if (spans.length === 0) {
    return (
      <div className="px-6 py-10 text-center">
        <p className="text-sm font-medium">No OpenTelemetry spans captured</p>
        <p className="mt-1 text-sm text-muted-foreground">
          Send OTLP spans with this run ID to see the execution waterfall.
        </p>
      </div>
    );
  }

  function toggle(key: string) {
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  function focusSpan(key: string) {
    setSelectedKey(key);
    rowRefs.current.get(key)?.focus();
  }

  function handleTreeKey(event: KeyboardEvent<HTMLButtonElement>, row: VisibleSpan) {
    const index = visible.findIndex((candidate) => candidate.span.key === row.span.key);
    if (index < 0) return;
    const previous = visible[index - 1];
    const next = visible[index + 1];
    const first = visible[0];
    const last = visible.at(-1);

    if (event.key === 'ArrowDown' && next) {
      event.preventDefault();
      focusSpan(next.span.key);
    } else if (event.key === 'ArrowUp' && previous) {
      event.preventDefault();
      focusSpan(previous.span.key);
    } else if (event.key === 'Home' && first) {
      event.preventDefault();
      focusSpan(first.span.key);
    } else if (event.key === 'End' && last) {
      event.preventDefault();
      focusSpan(last.span.key);
    } else if (event.key === 'ArrowRight' && row.childCount > 0) {
      event.preventDefault();
      if (collapsed.has(row.span.key)) toggle(row.span.key);
      else if (next?.parentKey === row.span.key) focusSpan(next.span.key);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      if (row.childCount > 0 && !collapsed.has(row.span.key)) toggle(row.span.key);
      else if (row.parentKey) focusSpan(row.parentKey);
    }
  }

  return (
    <div>
      <div className="overflow-x-auto" aria-label="OpenTelemetry span waterfall">
        <div className="min-w-4xl">
          <div className="grid grid-cols-5 border-b bg-muted/20 text-xs text-muted-foreground">
            <div className="col-span-2 px-4 py-3 font-medium">Span</div>
            <div className="col-span-3 border-l px-4 py-3">
              <div className="grid grid-cols-5 font-mono tabular-nums">
                <span>0</span>
                <span className="text-center">{formatDuration(domain.durationMs * 0.25)}</span>
                <span className="text-center">{formatDuration(domain.durationMs * 0.5)}</span>
                <span className="text-center">{formatDuration(domain.durationMs * 0.75)}</span>
                <span className="text-right">{formatDuration(domain.durationMs)}</span>
              </div>
            </div>
          </div>

          <div role="tree" aria-label="Run spans" aria-multiselectable="false">
            {visible.map((row) => {
              const isSelected = selected?.key === row.span.key;
              const isCollapsed = collapsed.has(row.span.key);
              const offset =
                ((row.span.startedMicros - domain.startMicros) / domain.durationMicros) * 100;
              const width = ((row.span.durationMs * 1_000) / domain.durationMicros) * 100;
              const tone =
                row.span.statusCode === 2
                  ? 'bg-destructive'
                  : (serviceTones.get(row.span.service) ?? 'bg-primary');

              return (
                <div
                  key={row.span.key}
                  role="treeitem"
                  aria-level={row.depth + 1}
                  aria-selected={isSelected}
                  aria-expanded={row.childCount > 0 ? !isCollapsed : undefined}
                  className={cn(
                    'grid grid-cols-5 border-b transition-colors last:border-b-0',
                    isSelected ? 'bg-muted/60' : 'hover:bg-muted/30',
                  )}
                >
                  <div className="col-span-2 flex min-w-0 items-center gap-1 px-3 py-2">
                    {Array.from({ length: row.depth }, (_, index) => (
                      <span key={index} className="w-4 shrink-0" aria-hidden="true" />
                    ))}
                    {row.childCount > 0 ? (
                      <button
                        type="button"
                        className="rounded-sm p-1 text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                        aria-label={`${isCollapsed ? 'Expand' : 'Collapse'} ${row.span.name}`}
                        onClick={() => toggle(row.span.key)}
                      >
                        {isCollapsed ? (
                          <ChevronRight className="size-3.5" aria-hidden="true" />
                        ) : (
                          <ChevronDown className="size-3.5" aria-hidden="true" />
                        )}
                      </button>
                    ) : (
                      <span className="size-7 shrink-0" aria-hidden="true" />
                    )}
                    <button
                      ref={(element) => {
                        if (element) rowRefs.current.set(row.span.key, element);
                        else rowRefs.current.delete(row.span.key);
                      }}
                      type="button"
                      className="min-w-0 flex-1 rounded-sm text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      onClick={() => setSelectedKey(row.span.key)}
                      onKeyDown={(event) => handleTreeKey(event, row)}
                    >
                      <span className="flex min-w-0 items-center gap-2">
                        <span className="truncate text-sm font-medium">{row.span.name}</span>
                        {row.span.statusCode === 2 ? (
                          <span className="text-xs font-medium text-destructive">Error</span>
                        ) : null}
                      </span>
                      <span className="block truncate text-xs text-muted-foreground">
                        {row.span.service} · {row.span.kind}
                      </span>
                    </button>
                  </div>

                  <button
                    type="button"
                    className="relative col-span-3 min-h-12 border-l bg-[linear-gradient(to_right,var(--border)_1px,transparent_1px)] bg-[length:20%_100%] px-4 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
                    aria-label={`Inspect ${row.span.name}, ${formatDuration(row.span.durationMs)}`}
                    onClick={() => setSelectedKey(row.span.key)}
                  >
                    <span
                      className={cn(
                        'absolute top-1/2 h-3 min-w-1 -translate-y-1/2 rounded-sm opacity-85',
                        tone,
                      )}
                      style={{ left: `${offset}%`, width: `${width}%` }}
                    />
                    <span
                      className="absolute top-1/2 -translate-y-1/2 pl-1 font-mono text-xs tabular-nums text-foreground"
                      style={{ left: `${Math.min(offset + width, 88)}%` }}
                    >
                      {formatDuration(row.span.durationMs)}
                    </span>
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {selected ? <SpanDetails span={selected} /> : null}
    </div>
  );
}

function SpanDetails({ span }: { span: RunSpan }) {
  return (
    <section className="border-t bg-muted/10 px-6 py-5" aria-labelledby="selected-span-title">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="text-xs font-medium text-muted-foreground">Selected span</p>
          <h3 id="selected-span-title" className="mt-1 truncate font-medium">
            {span.name}
          </h3>
          <p className="mt-1 text-sm text-muted-foreground">
            {span.service} · {span.kind} · {formatDuration(span.durationMs)}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant={span.statusCode === 2 ? 'destructive' : 'secondary'}>{span.status}</Badge>
          <Badge variant="outline">{span.contentCaptureStatus}</Badge>
          {span.lateEvidence ? <Badge variant="outline">Late evidence</Badge> : null}
        </div>
      </div>

      {span.statusMessage ? (
        <p className="mt-4 rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
          {span.statusMessage}
        </p>
      ) : null}

      <dl className="mt-5 grid gap-x-6 gap-y-4 sm:grid-cols-2 lg:grid-cols-3">
        <SpanDetail label="Trace ID" value={span.traceId} copy />
        <SpanDetail label="Span ID" value={span.spanId} copy />
        <SpanDetail
          label="Parent span"
          value={span.parentSpanId ?? 'Root span'}
          copy={span.parentSpanId !== null}
        />
        <SpanDetail label="Agent ID" value={span.agentId} copy />
        <SpanDetail label="Operation" value={span.operation ?? 'Not recorded'} />
        <SpanDetail label="Started" value={formatTimestamp(span.startedAt)} />
        <SpanDetail label="Events" value={String(span.eventCount)} />
        <SpanDetail label="Links" value={String(span.linkCount)} />
        <SpanDetail label="Dropped attributes" value={String(span.droppedAttributeCount)} />
      </dl>

      <div className="mt-5 grid gap-4 lg:grid-cols-2">
        <EvidenceList title="Span attributes" entries={span.attributes} />
        <EvidenceList title="Resource" entries={span.resource} />
      </div>
    </section>
  );
}

function SpanDetail({
  label,
  value,
  copy = false,
}: {
  label: string;
  value: string;
  copy?: boolean;
}) {
  return (
    <div className="min-w-0">
      <dt className="text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="mt-1 flex min-w-0 items-center gap-1.5 text-sm">
        <code className="truncate">{value}</code>
        {copy ? <CopyValueButton label={label} value={value} /> : null}
      </dd>
    </div>
  );
}

function CopyValueButton({ label, value }: { label: string; value: string }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1_500);
    } catch {
      toast.error(`Could not copy ${label.toLowerCase()}`);
    }
  }

  return (
    <button
      type="button"
      className="shrink-0 rounded-sm p-1 text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      aria-label={`Copy ${label.toLowerCase()} ${value}`}
      onClick={() => void copy()}
    >
      {copied ? (
        <Check className="size-3.5" aria-hidden="true" />
      ) : (
        <Copy className="size-3.5" aria-hidden="true" />
      )}
    </button>
  );
}

function EvidenceList({
  title,
  entries,
}: {
  title: string;
  entries: Array<{ label: string; value: string }>;
}) {
  return (
    <div className="rounded-lg border bg-card">
      <h4 className="border-b px-3 py-2 text-xs font-medium text-muted-foreground">{title}</h4>
      {entries.length === 0 ? (
        <p className="px-3 py-4 text-sm text-muted-foreground">None recorded</p>
      ) : (
        <dl className="max-h-52 overflow-y-auto divide-y">
          {entries.map((entry) => (
            <div key={entry.label} className="grid grid-cols-3 gap-3 px-3 py-2 text-xs">
              <dt className="truncate font-mono text-muted-foreground" title={entry.label}>
                {entry.label}
              </dt>
              <dd className="col-span-2 break-words font-mono">{entry.value}</dd>
            </div>
          ))}
        </dl>
      )}
    </div>
  );
}

function buildSpanTree(spans: RunDetailSnapshot['spans']): SpanNode[] {
  const nodes = new Map<string, SpanNode>();
  for (const span of spans) nodes.set(span.key, { span, parentKey: null, children: [] });

  const roots: SpanNode[] = [];
  for (const node of nodes.values()) {
    const parentKey = node.span.parentSpanId
      ? `${node.span.traceId}:${node.span.parentSpanId}`
      : null;
    const parent = parentKey ? nodes.get(parentKey) : null;
    if (parent && parent.span.key !== node.span.key) {
      node.parentKey = parent.span.key;
      parent.children.push(node);
    } else {
      roots.push(node);
    }
  }

  const byStart = (left: SpanNode, right: SpanNode) =>
    left.span.startedMicros - right.span.startedMicros ||
    left.span.key.localeCompare(right.span.key);
  roots.sort(byStart);
  for (const node of nodes.values()) node.children.sort(byStart);
  return roots;
}

function visibleSpans(roots: SpanNode[], collapsed: Set<string>): VisibleSpan[] {
  const rows: VisibleSpan[] = [];
  const visited = new Set<string>();

  function visit(node: SpanNode, depth: number) {
    if (visited.has(node.span.key)) return;
    visited.add(node.span.key);
    rows.push({
      span: node.span,
      parentKey: node.parentKey,
      depth,
      childCount: node.children.length,
    });
    if (!collapsed.has(node.span.key)) {
      for (const child of node.children) visit(child, depth + 1);
    }
  }

  for (const root of roots) visit(root, 0);
  return rows;
}

function spanDomain(
  spans: RunDetailSnapshot['spans'],
): { startMicros: number; durationMicros: number; durationMs: number } {
  const startMicros = Math.min(...spans.map((span) => span.startedMicros));
  const endMicros = Math.max(...spans.map((span) => span.endedMicros));
  const durationMicros = Math.max(1, endMicros - startMicros);
  return { startMicros, durationMicros, durationMs: durationMicros / 1_000 };
}

function serviceToneMap(spans: RunDetailSnapshot['spans']): Map<string, string> {
  const services = [...new Set(spans.map((span) => span.service))].sort();
  return new Map(
    services.map((service, index) => [
      service,
      SERVICE_TONES[index % SERVICE_TONES.length] ?? SERVICE_TONES[0],
    ]),
  );
}

function formatDuration(milliseconds: number): string {
  if (milliseconds < 1) return `${Math.round(milliseconds * 1_000)}µs`;
  if (milliseconds < 1_000) return `${Math.round(milliseconds)}ms`;
  return `${(milliseconds / 1_000).toFixed(milliseconds < 10_000 ? 2 : 1)}s`;
}

function formatTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isFinite(date.getTime())
    ? new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'medium' }).format(date)
    : value;
}
