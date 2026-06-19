'use client';

import { useCallback, useMemo, useState } from 'react';
import { ChevronRight, ShieldAlert } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import {
  RefreshControls,
  useAutoRefresh,
  type RefreshMode,
} from '@/components/workspace/RefreshControls';
import {
  parseRunDetailSnapshot,
  type RunDetailSnapshot,
} from '@/lib/run-detail-live';
import { cn } from '@/lib/utils';

type RunEvent = RunDetailSnapshot['events'][number];
type RunTrace = RunDetailSnapshot['traces'][number];

type TimelineRow =
  | {
      kind: 'trace';
      id: string;
      timestamp: number;
      order: number;
      trace: RunTrace;
      turn: { sequence: number; label: string } | null;
    }
  | { kind: 'event'; id: string; timestamp: number; order: number; event: RunEvent };

// 4-column event-log grid: time / type tag / summary / verdict. Shared by the
// sticky header and every row so columns stay aligned.
const ROW_GRID =
  'grid grid-cols-[4.75rem_minmax(0,1fr)_auto] gap-3 md:grid-cols-[5.5rem_9.5rem_minmax(0,1fr)_auto]';

export function RunDetailLiveView({
  initialData,
  runId,
  workspaceSlug,
}: {
  initialData: RunDetailSnapshot;
  runId: string;
  workspaceSlug: string;
}) {
  const [snapshot, setSnapshot] = useState(initialData);
  const [lastSync, setLastSync] = useState<Date>(() => new Date());
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<RefreshMode>('live');
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());

  const refresh = useCallback(async () => {
    const params = new URLSearchParams({ workspace: workspaceSlug });
    setIsRefreshing(true);

    try {
      const response = await fetch(
        `/api/runs/${encodeURIComponent(runId)}?${params.toString()}`,
        { cache: 'no-store' },
      );
      if (!response.ok) {
        throw new Error(`run refresh failed with ${response.status}`);
      }

      setSnapshot(parseRunDetailSnapshot(await response.json()));
      setLastSync(new Date());
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'run refresh failed');
    } finally {
      setIsRefreshing(false);
    }
  }, [runId, workspaceSlug]);

  useAutoRefresh(refresh, mode);

  const rows = useMemo(() => buildRows(snapshot), [snapshot]);

  const toggle = useCallback((id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const { run } = snapshot;
  const running = run.status.toLowerCase() === 'running';

  return (
    <div className="grid gap-4">
      {/* Identity band: who/what this run is, plus the live refresh controls. */}
      <Card className="gap-4 py-4">
        <CardHeader className="gap-3">
          <div className="flex min-w-0 flex-col gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <CardTitle className="font-mono text-lg tracking-tight">{run.shortId}</CardTitle>
              {/* Run status is a neutral, non-verdict concept — keep it gray (matching
                  the /runs list) so verdict tokens stay exclusive to allow/block/
                  rewrite/escalate. Liveness is carried by the pulse dot alone. */}
              <Badge variant="secondary" className="gap-1.5 font-mono text-[0.7rem]">
                <span
                  className={cn(
                    'size-1.5 rounded-full',
                    running
                      ? 'animate-pulse bg-current motion-reduce:animate-none'
                      : 'bg-muted-foreground/60',
                  )}
                />
                {run.status}
              </Badge>
            </div>
            <CardDescription className="font-mono text-xs">{run.agent}</CardDescription>
          </div>
          <div className="md:justify-self-end">
            <RefreshControls
              mode={mode}
              onModeChange={setMode}
              onRefresh={() => void refresh()}
              isRefreshing={isRefreshing}
              lastSync={lastSync}
              error={error}
            />
          </div>
        </CardHeader>
      </Card>

      {/* Outcome ledger: total checks, then the intervention counts that matter. */}
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
        <OutcomeStat label="Checks" value={run.traces} hint="Guardrail evaluations" />
        <OutcomeStat label="Blocked" value={run.blocked} tone="block" />
        <OutcomeStat label="Rewritten" value={run.rewritten} tone="rewrite" />
        <OutcomeStat label="Escalated" value={run.escalated} tone="escalate" />
        <OutcomeStat label="p95 latency" value={run.latency} hint="Across all checks" />
      </div>

      <Card className="gap-4 py-4">
        <CardHeader>
          <CardTitle className="text-sm">Run details</CardTitle>
          <CardDescription>Identifiers and timing for this session.</CardDescription>
        </CardHeader>
        <CardContent>
          <dl className="grid gap-x-6 gap-y-4 sm:grid-cols-2 lg:grid-cols-3">
            <DetailItem label="Kind" value={run.kind} />
            <DetailItem label="External ID" value={run.externalId} mono />
            <DetailItem label="Started" value={run.startedAt} />
            <DetailItem label="Ended" value={run.endedAt} />
            <DetailItem
              label="Run ID"
              value={run.id}
              mono
              className="sm:col-span-2 lg:col-span-3"
            />
            {run.metadata.map((item) => (
              <DetailItem key={item.label} label={item.label} value={item.value} />
            ))}
          </dl>
        </CardContent>
      </Card>

      <Card className="overflow-hidden gap-4 pt-4 pb-0">
        <CardHeader>
          <CardTitle className="text-sm">Live timeline</CardTitle>
          <CardDescription>
            Every guardrail check on this run, newest first. Select a row for the checked text and
            the policy that fired. Refreshes while this page is open.
          </CardDescription>
        </CardHeader>
        <CardContent className="px-0">
          {rows.length === 0 ? (
            <div className="px-6 pb-6">
              <TimelineEmptyState />
            </div>
          ) : (
            <div className="max-h-[60vh] overflow-y-auto border-t">
              <div
                className={cn(
                  ROW_GRID,
                  'sticky top-0 z-10 border-b bg-card/95 px-4 py-2 text-[10px] font-medium uppercase tracking-wider text-muted-foreground backdrop-blur',
                )}
              >
                <span>Time</span>
                <span className="hidden md:block">Type</span>
                <span>Summary</span>
                <span className="text-right">Verdict</span>
              </div>
              {rows.map((row) =>
                row.kind === 'trace' ? (
                  <TraceRow
                    key={row.id}
                    row={row}
                    open={expanded.has(row.id)}
                    onToggle={() => toggle(row.id)}
                  />
                ) : (
                  <EventRow
                    key={row.id}
                    row={row}
                    open={expanded.has(row.id)}
                    onToggle={() => toggle(row.id)}
                  />
                ),
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function TimelineEmptyState() {
  return (
    <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed px-6 py-10 text-center">
      <span className="flex size-9 items-center justify-center rounded-full border bg-muted/40 text-muted-foreground">
        <ShieldAlert className="size-4" />
      </span>
      <p className="text-sm font-medium">Waiting for the first check</p>
      <p className="max-w-sm text-xs text-muted-foreground">
        Guardrail checks appear here as the agent runs. This view refreshes on its own while open.
      </p>
    </div>
  );
}

function OutcomeStat({
  label,
  value,
  tone,
  hint,
}: {
  label: string;
  value: string | number;
  tone?: Outcome | undefined;
  hint?: string;
}) {
  const palette = tone ? OUTCOME_TONE[tone] : null;
  const active = tone != null && typeof value === 'number' && value > 0;

  return (
    <Card
      className={cn(
        'gap-1 px-4 py-3 shadow-none transition-colors',
        active && palette ? cn('border-l-2', palette.border) : 'border-l-2 border-l-transparent',
      )}
    >
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <div
        className={cn(
          'font-data text-2xl tabular-nums leading-none',
          active && palette ? palette.text : 'text-foreground',
        )}
      >
        {value}
      </div>
      {hint ? <div className="text-[11px] text-muted-foreground">{hint}</div> : null}
    </Card>
  );
}

function DetailItem({
  label,
  value,
  className,
  mono,
}: {
  label: string;
  value: string;
  className?: string;
  mono?: boolean;
}) {
  return (
    <div className={className}>
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className={cn('mt-1 break-words text-sm', mono && 'font-mono text-xs')}>{value}</dd>
    </div>
  );
}

function TraceRow({
  row,
  open,
  onToggle,
}: {
  row: Extract<TimelineRow, { kind: 'trace' }>;
  open: boolean;
  onToggle: () => void;
}) {
  const { trace, turn } = row;
  const tone = OUTCOME_TONE[normalizeOutcome(trace.outcome)];

  return (
    <div className="border-b last:border-b-0">
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={open}
        className={cn(
          ROW_GRID,
          'w-full items-center px-4 py-2.5 text-left transition-colors hover:bg-muted/50',
        )}
      >
        <TimeCell clock={trace.clock} time={trace.time} />

        <div className="hidden min-w-0 md:block">
          <TypeTag tone={tone} label={sideLabel(trace)} />
          {turn ? (
            <div className="mt-1 truncate text-[10px] text-muted-foreground">{turn.label}</div>
          ) : null}
        </div>

        <div className="flex min-w-0 items-center gap-2">
          <ChevronRight
            className={cn(
              'size-3.5 shrink-0 text-muted-foreground transition-transform',
              open && 'rotate-90',
            )}
          />
          <span className="truncate text-sm">{traceSummary(trace, tone)}</span>
        </div>

        <div className="justify-self-end">
          <VerdictPill outcome={trace.outcome} />
        </div>
      </button>

      {open ? <TraceDetail trace={trace} turn={turn} /> : null}
    </div>
  );
}

function TraceDetail({
  trace,
  turn,
}: {
  trace: RunTrace;
  turn: { sequence: number; label: string } | null;
}) {
  if (isDeliveryIntervention(trace)) {
    return <DeliveryInterventionDetail trace={trace} turn={turn} />;
  }

  const tone = OUTCOME_TONE[normalizeOutcome(trace.outcome)];
  return (
    <div className="border-t bg-muted/20 px-4 py-3 md:pl-[6.5rem]">
      <div className="mb-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground md:hidden">
        <TypeTag tone={tone} label={sideLabel(trace)} />
        {turn ? <span>{turn.label}</span> : null}
      </div>

      {trace.triggered ? (
        <div
          className={cn(
            'mb-3 flex items-start gap-2 rounded-md border-l-2 bg-muted/40 px-3 py-2 text-xs',
            tone.border,
          )}
        >
          <ShieldAlert className={cn('mt-0.5 size-3.5 shrink-0', tone.text)} />
          <div className="min-w-0">
            <span className="font-medium">
              {tone.label} by <span className="font-mono">{trace.policy}</span>
            </span>
            {trace.severity ? (
              <span className="text-muted-foreground"> · {trace.severity} severity</span>
            ) : null}
            {trace.reason ? (
              <div className="mt-0.5 break-words text-muted-foreground">{trace.reason}</div>
            ) : null}
          </div>
        </div>
      ) : trace.reason ? (
        <p className="mb-3 text-xs text-muted-foreground">{trace.reason}</p>
      ) : null}

      {trace.checkedInput ? <Excerpt label="Checked input" value={trace.checkedInput} /> : null}
      {trace.checkedOutput ? <Excerpt label="Checked output" value={trace.checkedOutput} /> : null}
      {trace.safeOutput ? <Excerpt label="Returned to caller" value={trace.safeOutput} /> : null}

      <TraceFooter phase={trace.phase} latency={trace.latency} id={trace.id} />
    </div>
  );
}

function DeliveryInterventionDetail({
  trace,
  turn,
}: {
  trace: RunTrace;
  turn: { sequence: number; label: string } | null;
}) {
  const outcome = normalizeOutcome(trace.outcome);
  const tone = OUTCOME_TONE[outcome];
  const stopped = outcome === 'block';
  const status = stopped
    ? 'TrustLoopGuard stopped this before delivery'
    : 'TrustLoopGuard rewrote this before delivery';
  const returned = trace.safeOutput ?? 'No unsafe response delivered';

  return (
    <div className="border-t bg-muted/20 px-4 py-3 md:pl-[6.5rem]">
      <div className="mb-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground md:hidden">
        <TypeTag tone={tone} label={sideLabel(trace)} />
        {turn ? <span>{turn.label}</span> : null}
      </div>

      <div
        className={cn(
          'mb-3 flex items-start gap-2 rounded-md border-l-2 bg-background px-3 py-2 text-xs',
          tone.border,
        )}
      >
        <ShieldAlert className={cn('mt-0.5 size-3.5 shrink-0', tone.text)} />
        <div className="min-w-0">
          <div className={cn('font-medium', tone.text)}>{status}</div>
          <div className="mt-0.5 break-words text-muted-foreground">
            <span className="font-mono">{trace.policy}</span>
            {trace.severity ? <span> · {trace.severity} severity</span> : null}
            {trace.reason && trace.reason !== 'No reason recorded' ? (
              <span> · {trace.reason}</span>
            ) : null}
          </div>
        </div>
      </div>

      {trace.checkedInput ? (
        <Excerpt label="User asked" value={displayUserPrompt(trace.checkedInput)} />
      ) : null}
      {trace.checkedOutput ? (
        <Excerpt label="Agent tried to say" value={trace.checkedOutput} />
      ) : null}
      <Excerpt label="TrustLoopGuard returned" value={returned} tone={tone} />

      <TraceFooter phase={trace.phase} latency={trace.latency} id={trace.id} />
    </div>
  );
}

function EventRow({
  row,
  open,
  onToggle,
}: {
  row: Extract<TimelineRow, { kind: 'event' }>;
  open: boolean;
  onToggle: () => void;
}) {
  const { event } = row;
  const summary =
    event.input !== 'No input summary'
      ? event.input
      : event.output !== 'No output summary'
        ? event.output
        : event.label;

  return (
    <div className="border-b last:border-b-0">
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={open}
        className={cn(
          ROW_GRID,
          'w-full items-center px-4 py-2.5 text-left transition-colors hover:bg-muted/50',
        )}
      >
        <TimeCell clock={event.clock} time={event.time} />
        <div className="hidden min-w-0 md:block">
          <TypeTag tone={OUTCOME_TONE.unknown} label={event.kind} />
        </div>
        <div className="flex min-w-0 items-center gap-2">
          <ChevronRight
            className={cn(
              'size-3.5 shrink-0 text-muted-foreground transition-transform',
              open && 'rotate-90',
            )}
          />
          <span className="truncate text-sm">{oneLine(summary)}</span>
        </div>
        <span className="justify-self-end font-data text-xs text-muted-foreground">
          #{event.sequence}
        </span>
      </button>

      {open ? (
        <div className="border-t bg-muted/20 px-4 py-3 md:pl-[6.5rem]">
          {event.input !== 'No input summary' ? (
            <Excerpt label="Input" value={event.input} />
          ) : null}
          {event.output !== 'No output summary' ? (
            <Excerpt label="Output" value={event.output} />
          ) : null}
          {event.metadata.length > 0 ? (
            <div className="mt-3 flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-muted-foreground">
              {event.metadata.map((item) => (
                <span key={item.label}>
                  {item.label}:{' '}
                  <span className="font-mono text-foreground">{item.value}</span>
                </span>
              ))}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function TimeCell({ clock, time }: { clock: string; time: string }) {
  return (
    <div className="leading-tight">
      <div className="font-data text-xs">{clock}</div>
      <div className="text-[10px] text-muted-foreground">{time}</div>
    </div>
  );
}

function TraceFooter({ phase, latency, id }: { phase: string; latency: string; id: string }) {
  return (
    <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
      <span>
        Phase: <span className="text-foreground/80">{phase}</span>
      </span>
      <Separator orientation="vertical" className="data-[orientation=vertical]:h-3" />
      <span>
        Latency: <span className="font-data text-foreground/80">{latency}</span>
      </span>
      <Separator orientation="vertical" className="data-[orientation=vertical]:h-3" />
      <span className="break-all font-mono">{id}</span>
    </div>
  );
}

function TypeTag({ tone, label }: { tone: Tone; label: string }) {
  return (
    <span
      className={cn(
        'inline-block max-w-full truncate rounded bg-current/10 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide',
        tone.text,
      )}
    >
      {label}
    </span>
  );
}

function VerdictPill({ outcome }: { outcome: string }) {
  const tone = OUTCOME_TONE[normalizeOutcome(outcome)];
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-xs font-medium',
        tone.border,
        tone.text,
      )}
    >
      <span className={cn('size-1.5 rounded-full', tone.dot)} />
      {tone.label}
    </span>
  );
}

function Excerpt({ label, value, tone }: { label: string; value: string; tone?: Tone }) {
  return (
    <div
      className={cn(
        'mt-2 rounded-md border bg-background p-2 first:mt-0',
        tone ? cn('border-l-2', tone.border) : undefined,
      )}
    >
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 whitespace-pre-wrap break-words font-mono text-xs">{value}</div>
    </div>
  );
}

function buildRows(snapshot: RunDetailSnapshot): TimelineRow[] {
  const eventById = new Map(snapshot.events.map((event) => [event.id, event]));
  const eventsWithTrace = new Set<string>();
  const rows: TimelineRow[] = [];
  let order = 0;

  for (const trace of snapshot.traces) {
    const event = trace.runEventId ? eventById.get(trace.runEventId) : undefined;
    if (event) eventsWithTrace.add(event.id);
    rows.push({
      kind: 'trace',
      id: trace.id,
      timestamp: trace.timestamp,
      order: order++,
      trace,
      turn: event ? { sequence: event.sequence, label: event.label } : null,
    });
  }

  for (const event of snapshot.events) {
    if (eventsWithTrace.has(event.id)) continue;
    rows.push({
      kind: 'event',
      id: event.id,
      timestamp: event.timestamp,
      order: order++,
      event,
    });
  }

  // Newest first; stable within equal timestamps by original chronological order.
  return rows.sort((a, b) => b.timestamp - a.timestamp || a.order - b.order);
}

function sideLabel(trace: RunTrace): string {
  if (trace.side === 'input') return 'Input check';
  if (trace.side === 'output') return 'Output check';
  return trace.phase;
}

function traceSummary(trace: RunTrace, tone: Tone): string {
  if (isDeliveryIntervention(trace)) {
    const verb = normalizeOutcome(trace.outcome) === 'block' ? 'Stopped' : 'Rewritten';
    return `${verb} before delivery · ${trace.policy}`;
  }

  if (trace.triggered) {
    const reason =
      trace.reason && trace.reason !== 'No reason recorded' ? ` — ${trace.reason}` : '';
    return `${tone.label} · ${trace.policy}${reason}`;
  }
  const text =
    trace.side === 'output'
      ? trace.checkedOutput ?? trace.safeOutput
      : trace.checkedInput ?? trace.checkedOutput;
  const summary = oneLine(text ?? '');
  if (summary) return summary;
  return trace.reason !== 'No reason recorded' ? trace.reason : 'No policy triggered';
}

function oneLine(value: string): string {
  return value.replace(/\s+/g, ' ').trim();
}

function isDeliveryIntervention(trace: RunTrace): boolean {
  const outcome = normalizeOutcome(trace.outcome);
  return (
    trace.side === 'output' &&
    trace.triggered &&
    (outcome === 'block' || outcome === 'rewrite')
  );
}

function displayUserPrompt(value: string): string {
  const text = value.trim();
  const lines = text
    .split(/\n+/)
    .map((line) => line.trim())
    .filter(Boolean);
  const latestUserLine = [...lines]
    .reverse()
    .find((line) => line.toLowerCase().startsWith('user:'));
  return (latestUserLine ? latestUserLine.replace(/^user:\s*/i, '') : text).trim();
}

type Outcome = 'allow' | 'block' | 'rewrite' | 'escalate' | 'unknown';

type Tone = { label: string; border: string; dot: string; text: string };

// Verdict colors reuse the canonical guardrail tokens from globals.css
// (--color-allow green, --color-block red, --color-rewrite amber, --color-escalate violet)
// so this view matches the dashboard decisions table.
const OUTCOME_TONE: Record<Outcome, Tone> = {
  allow: {
    label: 'Allowed',
    border: 'border-[color:var(--color-allow)]',
    dot: 'bg-[color:var(--color-allow)]',
    text: 'text-[color:var(--color-allow)]',
  },
  block: {
    label: 'Blocked',
    border: 'border-[color:var(--color-block)]',
    dot: 'bg-[color:var(--color-block)]',
    text: 'text-[color:var(--color-block)]',
  },
  rewrite: {
    label: 'Rewritten',
    border: 'border-[color:var(--color-rewrite)]',
    dot: 'bg-[color:var(--color-rewrite)]',
    text: 'text-[color:var(--color-rewrite)]',
  },
  escalate: {
    label: 'Escalated',
    border: 'border-[color:var(--color-escalate)]',
    dot: 'bg-[color:var(--color-escalate)]',
    text: 'text-[color:var(--color-escalate)]',
  },
  unknown: {
    label: 'Checked',
    border: 'border-border',
    dot: 'bg-muted-foreground',
    text: 'text-muted-foreground',
  },
};

function normalizeOutcome(outcome: string): Outcome {
  const lower = outcome.toLowerCase();
  if (lower === 'allow' || lower === 'block' || lower === 'rewrite' || lower === 'escalate') {
    return lower;
  }
  return 'unknown';
}
