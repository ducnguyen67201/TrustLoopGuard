'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Check, ChevronRight, Copy, ShieldAlert } from 'lucide-react';
import Link from 'next/link';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { InfoHint } from '@/components/ui/info-hint';
import { Separator } from '@/components/ui/separator';
import { AuthorizationEffectLegend } from '@/components/ui/authorization-effect-legend';
import {
  RefreshControls,
  useAutoRefresh,
  type RefreshMode,
} from '@/components/workspace/RefreshControls';
import {
  formatUsdNanos,
  parseRunDetailSnapshot,
  type RunAgentIdentity,
  type RunDetailSnapshot,
} from '@/lib/run-detail-live';
import { cn } from '@/lib/utils';

type RunEvent = RunDetailSnapshot['events'][number];
type RunTrace = RunDetailSnapshot['traces'][number];
type FlowStepTone = 'neutral' | Outcome;

type GuardFlowStep = {
  title: string;
  subtitle: string;
  badge: string;
  tone: FlowStepTone;
};

type TraceTurn = {
  kind: string;
  label: string;
  output: string;
};

type TimelineRow =
  | {
      kind: 'trace';
      id: string;
      timestamp: number;
      order: number;
      trace: RunTrace;
      turn: TraceTurn | null;
    }
  | { kind: 'event'; id: string; timestamp: number; order: number; event: RunEvent };

// 4-column event-log grid: time / type tag / summary / effect. Shared by the
// sticky header and every row so columns stay aligned.
const ROW_GRID =
  'grid grid-cols-[4.75rem_minmax(0,1fr)_auto] gap-3 md:grid-cols-[5.5rem_9.5rem_minmax(0,1fr)_auto]';

export function RunDetailLiveView({
  initialData,
  runId,
  workspaceSlug,
  agentIdentity,
}: {
  initialData: RunDetailSnapshot;
  runId: string;
  workspaceSlug: string;
  agentIdentity: RunAgentIdentity;
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
      const response = await fetch(`/api/runs/${encodeURIComponent(runId)}?${params.toString()}`, {
        cache: 'no-store',
      });
      if (!response.ok) {
        throw new Error(`run refresh failed with ${response.status}`);
      }

      setSnapshot(parseRunDetailSnapshot(await response.json()));
      setLastSync(new Date());
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'run refresh failed');
      setMode('manual');
    } finally {
      setIsRefreshing(false);
    }
  }, [runId, workspaceSlug]);

  useAutoRefresh(refresh, mode);

  const rows = useMemo(() => buildRows(snapshot), [snapshot]);
  const guardFlow = useMemo(() => buildGuardFlow(snapshot), [snapshot]);

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
      <p className="text-sm text-muted-foreground [text-wrap:pretty]">
        This is the full story of one request your agent sent through the guardrail — every check it
        ran, what it looked at, and the decision it reached. Read the timeline from the top for the
        latest step.
      </p>

      {/* Identity band: who/what this run is, plus the live refresh controls. */}
      <Card className="gap-4 py-4">
        <CardHeader className="gap-3">
          <div className="flex min-w-0 flex-col gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <CardTitle className="font-mono text-lg tracking-tight">{run.shortId}</CardTitle>
              {/* Run status is a neutral, non-effect concept — keep it gray (matching
                  the /runs list) so effect tokens stay exclusive to the canonical
                  authorization effects. Liveness is carried by the pulse dot alone. */}
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
            <div className="grid gap-1.5">
              <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm">
                <span className="text-xs text-muted-foreground">Registered agent</span>
                {agentIdentity.displayName && agentIdentity.href ? (
                  <Link
                    href={agentIdentity.href}
                    className="inline-flex min-w-0 items-center gap-1 font-medium text-foreground underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                  >
                    <span className="truncate">{agentIdentity.displayName}</span>
                    <ChevronRight className="size-3.5 shrink-0" aria-hidden="true" />
                  </Link>
                ) : (
                  <span className="font-medium text-muted-foreground">Agent unavailable</span>
                )}
              </div>
              <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                <span className="text-xs text-muted-foreground">Agent ID</span>
                <CopyIdButton id={agentIdentity.id} label="agent" truncate={false} />
              </div>
            </div>
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
        <OutcomeStat
          label="Checks"
          value={run.traces}
          hint="Guardrail reviews"
          info="How many times the guardrail looked at this request — once per input and output it reviewed."
        />
        <OutcomeStat
          label="Denied"
          value={run.blocked}
          tone="deny"
          info="Checks that were stopped because they broke one of your rules."
        />
        <OutcomeStat
          label="Transformed"
          value={run.rewritten}
          tone="transform"
          info="Checks the guardrail cleaned up, then let through."
        />
        <OutcomeStat
          label="Approval required"
          value={run.escalated}
          tone="require_approval"
          info="Checks held for a person to review before continuing."
        />
        <OutcomeStat
          label="Speed"
          value={run.latency}
          hint="Typical check time"
          info="How long the guardrail took to check this request, in milliseconds. Lower is faster."
        />
      </div>

      <Card className="gap-4 py-4">
        <CardHeader>
          <CardTitle className="text-sm">About this request</CardTitle>
          <CardDescription>
            When it ran and the references you can use to look it up.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <dl className="grid gap-x-6 gap-y-4 sm:grid-cols-2 lg:grid-cols-3">
            <DetailItem label="Type" value={run.kind} />
            <DetailItem label="Your label" value={run.externalId} mono />
            <DetailItem label="Started" value={run.startedAt} />
            <DetailItem label="Ended" value={run.endedAt} />
            <DetailItem
              label="Request ID"
              value={run.id}
              copyId
              className="sm:col-span-2 lg:col-span-3"
            />
            {run.metadata.map((item) => (
              <DetailItem key={item.label} label={item.label} value={item.value} />
            ))}
          </dl>
        </CardContent>
      </Card>

      <div className="grid gap-3 lg:grid-cols-3">
        <ProviderUsageCard usage={snapshot.providerUsage} />
        <BudgetDecisionCard decision={snapshot.budgetDecision} />
        <GuardrailUsageCard usage={snapshot.guardrailUsage} />
      </div>

      <Card className="gap-4 py-4">
        <CardHeader className="gap-1">
          <CardTitle className="text-sm">Guard flow</CardTitle>
          <CardDescription>
            How the agent turn moved through logging, output checks, and protected actions.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ol className="grid gap-2 md:grid-cols-5">
            {guardFlow.map((step, index) => (
              <GuardFlowItem key={step.title} step={step} index={index} />
            ))}
          </ol>
        </CardContent>
      </Card>

      <Card className="overflow-hidden gap-4 pt-4 pb-0">
        <CardHeader className="gap-3">
          <div className="grid gap-1">
            <CardTitle className="text-sm">Agent events and guard checks</CardTitle>
            <CardDescription>
              Transcript events and TrustLoopGuard decisions, newest first. Click any row to see the
              exact text, linked guard check, and policy outcome.
            </CardDescription>
          </div>
          <div className="rounded-lg border bg-muted/20 px-4 py-3">
            <p className="mb-2.5 text-xs font-medium text-muted-foreground">
              What each effect color means
            </p>
            <AuthorizationEffectLegend />
          </div>
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
                <span className="inline-flex items-center gap-1">
                  Time
                  <InfoHint label="What does “Time” mean?">
                    When this check happened. The smaller line is how long ago.
                  </InfoHint>
                </span>
                <span className="hidden items-center gap-1 md:inline-flex">
                  Step
                  <InfoHint label="What does “Step” mean?">
                    What the guardrail was checking — the request going in, or the agent’s reply
                    coming out.
                  </InfoHint>
                </span>
                <span className="inline-flex items-center gap-1">
                  What happened
                  <InfoHint label="What does “What happened” mean?">
                    A one-line plain-language summary of this step. Click the row for the full
                    details.
                  </InfoHint>
                </span>
                <span className="inline-flex items-center justify-end gap-1 text-right">
                  Decision
                  <InfoHint label="What does “Decision” mean?" side="left">
                    What the guardrail decided: permitted, transformed, approval required, deferred,
                    or denied. Colors are explained in the key above.
                  </InfoHint>
                </span>
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

function GuardFlowItem({ step, index }: { step: GuardFlowStep; index: number }) {
  const tone = step.tone === 'neutral' ? null : OUTCOME_TONE[step.tone];

  return (
    <li
      className={cn(
        'grid min-w-0 gap-2 rounded-lg border bg-muted/15 p-3',
        tone ? cn('border-l-2', tone.border) : 'border-l-2 border-l-transparent',
      )}
    >
      <div className="flex min-w-0 items-center gap-2">
        <span className="flex size-6 shrink-0 items-center justify-center rounded-md border bg-background font-mono text-[11px] text-muted-foreground">
          {index + 1}
        </span>
        <span
          className={cn(
            'min-w-0 truncate text-xs font-semibold',
            tone ? tone.text : 'text-foreground',
          )}
        >
          {step.title}
        </span>
      </div>
      <p className="min-w-0 text-xs leading-relaxed text-muted-foreground">{step.subtitle}</p>
      <span
        className={cn(
          'w-fit rounded-full border px-2 py-0.5 font-mono text-[10px] uppercase tracking-wide',
          tone ? cn(tone.border, tone.text) : 'text-muted-foreground',
        )}
      >
        {step.badge}
      </span>
    </li>
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
  info,
}: {
  label: string;
  value: string | number;
  tone?: Outcome | undefined;
  hint?: string;
  info?: string;
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
      <div className="flex items-center gap-1 text-xs font-medium text-muted-foreground">
        {label}
        {info ? <InfoHint label={`What does “${label}” mean?`}>{info}</InfoHint> : null}
      </div>
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
  copyId,
}: {
  label: string;
  value: string;
  className?: string;
  mono?: boolean;
  copyId?: boolean;
}) {
  return (
    <div className={className}>
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="mt-1">
        {copyId ? (
          <CopyIdButton id={value} label="request" />
        ) : (
          <span className={cn('break-words text-sm', mono && 'font-mono text-xs')}>{value}</span>
        )}
      </dd>
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
          <span className="truncate text-sm">{traceSummary(trace, tone, turn)}</span>
        </div>

        <div className="justify-self-end">
          <EffectPill outcome={trace.outcome} />
        </div>
      </button>

      {open ? <TraceDetail trace={trace} turn={turn} /> : null}
    </div>
  );
}

function TraceDetail({ trace, turn }: { trace: RunTrace; turn: TraceTurn | null }) {
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
              {tone.label} by{' '}
              <span className={isParameterAccountSourceFailure(trace) ? undefined : 'font-mono'}>
                {displayPolicy(trace)}
              </span>
            </span>
            {trace.severity ? (
              <span className="text-muted-foreground"> · {trace.severity} severity</span>
            ) : null}
            {displayReason(trace) ? (
              <div className="mt-0.5 break-words text-muted-foreground">{displayReason(trace)}</div>
            ) : null}
          </div>
        </div>
      ) : displayReason(trace) ? (
        <p className="mb-3 text-xs text-muted-foreground">{displayReason(trace)}</p>
      ) : null}

      {trace.checkedInput ? (
        <Excerpt
          label={trace.side === 'tool' ? 'Proposed tool parameters' : 'Checked input'}
          value={trace.checkedInput}
        />
      ) : null}
      {trace.checkedOutput ? <Excerpt label="Checked output" value={trace.checkedOutput} /> : null}
      {!trace.checkedOutput && linkedEventOutput(turn) ? (
        <Excerpt label={linkedEventOutputLabel(turn)} value={linkedEventOutput(turn)!} />
      ) : null}
      {trace.safeOutput ? <Excerpt label="Returned to caller" value={trace.safeOutput} /> : null}

      <TraceFooter side={trace.side} latency={trace.latency} id={trace.id} />
    </div>
  );
}

function DeliveryInterventionDetail({ trace, turn }: { trace: RunTrace; turn: TraceTurn | null }) {
  const outcome = normalizeOutcome(trace.outcome);
  const tone = OUTCOME_TONE[outcome];
  const stopped = outcome === 'deny';
  const status = stopped
    ? 'TrustLoopGuard stopped this before delivery'
    : 'TrustLoopGuard transformed this before delivery';
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
            <span className={isParameterAccountSourceFailure(trace) ? undefined : 'font-mono'}>
              {displayPolicy(trace)}
            </span>
            {trace.severity ? <span> · {trace.severity} severity</span> : null}
            {displayReason(trace) ? <span> · {displayReason(trace)}</span> : null}
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

      <TraceFooter side={trace.side} latency={trace.latency} id={trace.id} />
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
  const summary = eventSummary(event);

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
                  {item.label}: <span className="font-mono text-foreground">{item.value}</span>
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

function TraceFooter({
  side,
  latency,
  id,
}: {
  side: RunTrace['side'];
  latency: string;
  id: string;
}) {
  return (
    <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
      <span className="inline-flex items-center gap-1">
        Stage: <span className="text-foreground/80">{stageLabel(side)}</span>
        <InfoHint label="What does “Stage” mean?">
          When in the request this check ran — before the AI replied (checking what came in) or
          after (checking what the AI was about to say).
        </InfoHint>
      </span>
      <Separator orientation="vertical" className="data-[orientation=vertical]:h-3" />
      <span className="inline-flex items-center gap-1">
        Took: <span className="font-data text-foreground/80">{latency}</span>
        <InfoHint label="What does this time mean?">
          How long the guardrail spent on this one check, in milliseconds. Lower is faster.
        </InfoHint>
      </span>
      <Separator orientation="vertical" className="data-[orientation=vertical]:h-3" />
      <span className="inline-flex min-w-0 items-center gap-1">
        <span className="text-muted-foreground/80">Check ID</span>
        <CopyIdButton id={id} label="check" />
      </span>
    </div>
  );
}

/** A one-tap copy for a long technical id, so it is shareable but never noise. */
function CopyIdButton({
  id,
  label,
  truncate = true,
}: {
  id: string;
  label: string;
  truncate?: boolean;
}) {
  const [copied, setCopied] = useState(false);
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (resetTimer.current) clearTimeout(resetTimer.current);
    },
    [],
  );

  async function copy() {
    try {
      await navigator.clipboard.writeText(id);
    } catch {
      toast.error("Couldn't copy — select the text and copy it manually.");
      return;
    }
    setCopied(true);
    if (resetTimer.current) clearTimeout(resetTimer.current);
    resetTimer.current = setTimeout(() => setCopied(false), 1600);
  }

  return (
    <button
      type="button"
      onClick={copy}
      aria-label={copied ? `${label} ID copied` : `Copy full ${label} ID ${id}`}
      className="inline-flex items-center gap-1 rounded-md px-1 py-0.5 font-mono text-muted-foreground/80 transition-colors hover:bg-accent hover:text-foreground focus-visible:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
    >
      <span className={cn(truncate ? 'max-w-[12rem] truncate' : 'break-all text-left')}>{id}</span>
      {copied ? (
        <Check className="size-3 text-[color:var(--color-permit)]" aria-hidden />
      ) : (
        <Copy className="size-3" aria-hidden />
      )}
    </button>
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

function EffectPill({ outcome }: { outcome: string }) {
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

function ProviderUsageCard({ usage }: { usage: RunDetailSnapshot['providerUsage'] }) {
  return (
    <Card className="gap-3 py-4">
      <CardHeader className="gap-1">
        <CardTitle className="text-sm">Provider call</CardTitle>
        <CardDescription>Customer inference usage and estimated provider cost.</CardDescription>
      </CardHeader>
      <CardContent>
        {usage ? (
          <dl className="grid gap-2 text-xs">
            <DetailItem label="Model" value={`${usage.provider} · ${usage.model}`} mono />
            <DetailItem label="Status" value={titleCase(usage.status)} />
            <DetailItem
              label="Tokens"
              value={
                usage.prompt_tokens === null || usage.completion_tokens === null
                  ? 'Unknown'
                  : `${usage.prompt_tokens.toLocaleString()} input · ${usage.completion_tokens.toLocaleString()} output`
              }
            />
            <DetailItem
              label="Estimated cost"
              value={formatUsdNanos(usage.estimated_cost_usd_nanos)}
            />
            <DetailItem
              label="Price snapshot"
              value={
                usage.input_rate_usd_per_million_nanos === null ||
                usage.output_rate_usd_per_million_nanos === null
                  ? 'Unknown'
                  : `${formatUsdNanos(usage.input_rate_usd_per_million_nanos)} input · ${formatUsdNanos(usage.output_rate_usd_per_million_nanos)} output per 1M`
              }
            />
            <DetailItem label="Provider latency" value={`${usage.latency_ms}ms`} />
            {usage.provider_response_id ? (
              <DetailItem label="Provider response" value={usage.provider_response_id} mono />
            ) : null}
          </dl>
        ) : (
          <p className="text-xs text-muted-foreground">
            Provider usage was not recorded for this historical run.
          </p>
        )}
      </CardContent>
    </Card>
  );
}

function BudgetDecisionCard({ decision }: { decision: RunDetailSnapshot['budgetDecision'] }) {
  const governing = decision?.governing_window ? ` · ${decision.governing_window}` : '';
  const softAdmission = decision?.status.startsWith('soft_') ?? false;
  return (
    <Card className="gap-3 py-4">
      <CardHeader className="gap-1">
        <CardTitle className="text-sm">Spending cap</CardTitle>
        <CardDescription>
          {softAdmission
            ? 'No output bound was provided; actual usage applies and may overshoot once.'
            : 'Deterministic admission before provider traffic.'}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {decision ? (
          <div className="grid gap-3 text-xs">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={decision.status === 'denied' ? 'destructive' : 'secondary'}>
                {titleCase(decision.status)}
                {governing}
              </Badge>
              <span className="font-mono text-muted-foreground">{decision.principal_id}</span>
            </div>
            {decision.windows.length > 0 ? (
              <dl className="grid gap-2">
                {decision.windows.map((window) => (
                  <div key={window.window} className="rounded-lg border p-2">
                    <dt className="font-medium capitalize">{window.window}</dt>
                    <dd className="mt-1 text-muted-foreground">
                      {formatUsdNanos(window.remaining_after_usd_nanos)} remaining of{' '}
                      {formatUsdNanos(window.cap_usd_nanos)}
                    </dd>
                    <dd className="mt-1 text-muted-foreground">
                      {softAdmission ? (
                        <>
                          {formatUsdNanos(window.committed_before_usd_nanos)} committed before ·{' '}
                          unbounded request settled to actual usage
                        </>
                      ) : (
                        <>
                          {formatUsdNanos(window.committed_before_usd_nanos)} committed before ·{' '}
                          {formatUsdNanos(window.reserved_before_usd_nanos)} already reserved ·{' '}
                          {formatUsdNanos(window.requested_usd_nanos)} maximum reserved
                        </>
                      )}
                    </dd>
                  </div>
                ))}
              </dl>
            ) : (
              <p className="text-muted-foreground">No LLM spending cap was configured.</p>
            )}
            {decision.actual_usd_nanos ? (
              <p>Actual charge: {formatUsdNanos(decision.actual_usd_nanos)}</p>
            ) : null}
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">
            Budget evidence was not recorded for this historical run.
          </p>
        )}
      </CardContent>
    </Card>
  );
}

function GuardrailUsageCard({ usage }: { usage: RunDetailSnapshot['guardrailUsage'] }) {
  const known = usage
    .map((item) => item.estimated_cost_usd_nanos)
    .filter((value): value is string => value !== null);
  const total = known.reduce((sum, value) => sum + BigInt(value), 0n).toString();
  return (
    <Card className="gap-3 py-4">
      <CardHeader className="gap-1">
        <CardTitle className="text-sm">Guardrail overhead</CardTitle>
        <CardDescription>
          TrustLoopGuard semantic checks, separate from customer spend.
        </CardDescription>
      </CardHeader>
      <CardContent>
        {usage.length > 0 ? (
          <div className="grid gap-2 text-xs">
            <p className="font-medium">
              {known.length === usage.length ? formatUsdNanos(total) : 'Partially unknown'}{' '}
              estimated
            </p>
            {usage.map((item, index) => (
              <div key={`${item.phase}-${item.judge}-${index}`} className="rounded-lg border p-2">
                <div className="font-medium">
                  {titleCase(item.phase)} · {item.judge}
                </div>
                <div className="mt-1 text-muted-foreground">
                  {item.model ?? 'No model'} · {formatUsdNanos(item.estimated_cost_usd_nanos)} ·{' '}
                  {item.latency_ms}ms
                </div>
                <div className="mt-1 text-muted-foreground">
                  {titleCase(item.status)} ·{' '}
                  {item.prompt_tokens === null || item.completion_tokens === null
                    ? 'tokens unknown'
                    : `${item.prompt_tokens.toLocaleString()} input · ${item.completion_tokens.toLocaleString()} output`}
                  {item.fallback_used ? ' · fallback used' : ''}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">
            Deterministic only — no guardrail LLM cost.
          </p>
        )}
      </CardContent>
    </Card>
  );
}

function titleCase(value: string): string {
  return value.replaceAll('_', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function buildRows(snapshot: RunDetailSnapshot): TimelineRow[] {
  const eventById = new Map(snapshot.events.map((event) => [event.id, event]));
  const rows: TimelineRow[] = [];
  let order = 0;

  for (const trace of snapshot.traces) {
    const event = trace.runEventId ? eventById.get(trace.runEventId) : undefined;
    rows.push({
      kind: 'trace',
      id: trace.id,
      timestamp: trace.timestamp,
      order: order++,
      trace,
      turn: event
        ? {
            kind: event.kind,
            label: event.label,
            output: event.output,
          }
        : null,
    });
  }

  for (const event of snapshot.events) {
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

function eventSummary(event: RunEvent): string {
  if (event.kind === 'Assistant Turn' && event.output !== 'No output summary') return event.output;
  if (event.kind === 'Tool Call' && event.output !== 'No output summary') return event.output;
  if (event.input !== 'No input summary') return event.input;
  if (event.output !== 'No output summary') return event.output;
  return event.label;
}

function buildGuardFlow(snapshot: RunDetailSnapshot): GuardFlowStep[] {
  const eventById = new Map(snapshot.events.map((event) => [event.id, event]));
  const userTurns = snapshot.events.filter((event) => event.kind === 'User Turn').length;
  const assistantTurns = snapshot.events.filter((event) => event.kind === 'Assistant Turn').length;
  const loggedToolCalls = snapshot.events.filter((event) => event.kind === 'Tool Call').length;
  const guardedToolCalls = snapshot.traces.filter((trace) => trace.side === 'tool').length;
  const toolCalls = Math.max(loggedToolCalls, guardedToolCalls);
  const outputChecks = snapshot.traces.filter((trace) => {
    const event = trace.runEventId ? eventById.get(trace.runEventId) : undefined;
    return event?.kind === 'Assistant Turn' || trace.side === 'output';
  });
  const actionChecks = snapshot.traces.filter((trace) => {
    const event = trace.runEventId ? eventById.get(trace.runEventId) : undefined;
    return (
      trace.side === 'tool' ||
      event?.kind === 'Tool Call' ||
      (trace.side === 'other' && event?.kind !== 'Assistant Turn')
    );
  });

  return [
    {
      title: 'User input',
      subtitle:
        userTurns > 0
          ? `${userTurns} user ${userTurns === 1 ? 'turn' : 'turns'} logged.`
          : 'No user turn logged yet.',
      badge: userTurns > 0 ? 'logged' : 'missing',
      tone: 'neutral',
    },
    {
      title: 'Agent draft',
      subtitle:
        assistantTurns > 0
          ? `${assistantTurns} assistant ${assistantTurns === 1 ? 'reply' : 'replies'} logged.`
          : 'No assistant draft logged yet.',
      badge: assistantTurns > 0 ? 'logged' : 'missing',
      tone: 'neutral',
    },
    {
      title: 'Output guard',
      subtitle: guardCheckSubtitle(outputChecks, 'assistant output'),
      badge: outputChecks.length > 0 ? effectSummary(outputChecks) : 'not checked',
      tone: worstTraceTone(outputChecks),
    },
    {
      title: 'Tool/action',
      subtitle:
        toolCalls > 0
          ? `${toolCalls} protected ${toolCalls === 1 ? 'action was' : 'actions were'} proposed.`
          : 'No protected tool or action was proposed.',
      badge: toolCalls > 0 ? 'proposed' : 'none',
      tone: 'neutral',
    },
    {
      title: 'Action guard',
      subtitle: guardCheckSubtitle(actionChecks, 'tool or action'),
      badge: actionChecks.length > 0 ? effectSummary(actionChecks) : 'not checked',
      tone: worstTraceTone(actionChecks),
    },
  ];
}

function guardCheckSubtitle(traces: RunTrace[], subject: string): string {
  if (traces.length === 0) return `No ${subject} guard check has run yet.`;
  return `${traces.length} ${subject} ${traces.length === 1 ? 'check' : 'checks'} ran.`;
}

function effectSummary(traces: RunTrace[]): string {
  const counts = traces.reduce(
    (acc, trace) => {
      acc[normalizeOutcome(trace.outcome)] += 1;
      return acc;
    },
    {
      permit: 0,
      deny: 0,
      transform: 0,
      require_approval: 0,
      defer: 0,
      unknown: 0,
    } satisfies Record<Outcome, number>,
  );
  const parts = (['deny', 'defer', 'require_approval', 'transform', 'permit'] as const)
    .filter((outcome) => counts[outcome] > 0)
    .map((outcome) => `${counts[outcome]} ${OUTCOME_TONE[outcome].label.toLowerCase()}`);
  return parts[0] ?? 'checked';
}

function worstTraceTone(traces: RunTrace[]): FlowStepTone {
  if (traces.some((trace) => normalizeOutcome(trace.outcome) === 'deny')) return 'deny';
  if (traces.some((trace) => normalizeOutcome(trace.outcome) === 'defer')) return 'defer';
  if (traces.some((trace) => normalizeOutcome(trace.outcome) === 'require_approval'))
    return 'require_approval';
  if (traces.some((trace) => normalizeOutcome(trace.outcome) === 'transform')) return 'transform';
  if (traces.some((trace) => normalizeOutcome(trace.outcome) === 'permit')) return 'permit';
  return 'neutral';
}

function sideLabel(trace: RunTrace): string {
  if (trace.side === 'input') return 'Input check';
  if (trace.side === 'output') return 'Output check';
  if (trace.side === 'tool') return 'Tool check';
  return trace.phase;
}

/**
 * Friendly, plain-language name for when in the request a check ran. The raw
 * value is a technical token (e.g. "Gateway Input Check") — this maps the
 * derived side to words a non-technical reader understands. Presentational only.
 */
function stageLabel(side: RunTrace['side']): string {
  if (side === 'input') return 'Before the AI replied';
  if (side === 'output') return 'After the AI replied';
  if (side === 'tool') return 'Before the tool ran';
  return 'During the request';
}

function traceSummary(trace: RunTrace, tone: Tone, turn: TraceTurn | null): string {
  if (isDeliveryIntervention(trace)) {
    const verb = normalizeOutcome(trace.outcome) === 'deny' ? 'Stopped' : 'Transformed';
    return `${verb} before delivery · ${trace.policy}`;
  }

  if (trace.triggered) {
    const reason = displayReason(trace);
    if (isParameterAccountSourceFailure(trace)) return reason ?? 'Denied unsafe refund account';
    return `${tone.label} · ${trace.policy}${reason ? ` — ${reason}` : ''}`;
  }
  if (trace.side === 'tool') {
    const operation = trace.operation ?? trace.toolName ?? 'Tool call';
    return `${titleCase(operation)} · ${tone.label} · ${trace.policy}`;
  }
  const text =
    trace.side === 'output'
      ? (trace.checkedOutput ?? trace.safeOutput)
      : (trace.checkedInput ?? trace.checkedOutput ?? linkedEventOutput(turn));
  const summary = oneLine(text ?? '');
  if (summary) return summary;
  return displayReason(trace) ?? 'No policy triggered';
}

function linkedEventOutput(turn: TraceTurn | null): string | null {
  if (turn?.kind !== 'Assistant Turn' && turn?.kind !== 'Tool Call') return null;
  return turn.output === 'No output summary' ? null : turn.output;
}

function linkedEventOutputLabel(turn: TraceTurn | null): string {
  return turn?.kind === 'Tool Call' ? 'Action checked' : 'Agent reply checked';
}

function oneLine(value: string): string {
  return value.replace(/\s+/g, ' ').trim();
}

function displayReason(trace: RunTrace): string | null {
  if (!trace.reason || trace.reason === 'No reason recorded') return null;
  if (isParameterAccountSourceFailure(trace)) {
    return 'Stopped because the refund account came from the chat, not a trusted account record.';
  }
  return trace.reason;
}

function displayPolicy(trace: RunTrace): string {
  if (isParameterAccountSourceFailure(trace)) return 'Refund account source';
  return trace.policy;
}

function isParameterAccountSourceFailure(trace: RunTrace): boolean {
  return (
    trace.policy === 'parameter_source.account' ||
    trace.reason?.includes("authority-bearing parameter 'account'") === true
  );
}

function isDeliveryIntervention(trace: RunTrace): boolean {
  const outcome = normalizeOutcome(trace.outcome);
  return (
    trace.side === 'output' && trace.triggered && (outcome === 'deny' || outcome === 'transform')
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

type Outcome = 'permit' | 'deny' | 'transform' | 'require_approval' | 'defer' | 'unknown';

type Tone = { label: string; border: string; dot: string; text: string };

// Effect colors reuse the canonical guardrail tokens from globals.css.
const OUTCOME_TONE: Record<Outcome, Tone> = {
  permit: {
    label: 'Permitted',
    border: 'border-[color:var(--color-permit)]',
    dot: 'bg-[color:var(--color-permit)]',
    text: 'text-[color:var(--color-permit)]',
  },
  deny: {
    label: 'Denied',
    border: 'border-[color:var(--color-deny)]',
    dot: 'bg-[color:var(--color-deny)]',
    text: 'text-[color:var(--color-deny)]',
  },
  transform: {
    label: 'Transformed',
    border: 'border-[color:var(--color-transform)]',
    dot: 'bg-[color:var(--color-transform)]',
    text: 'text-[color:var(--color-transform)]',
  },
  require_approval: {
    label: 'Approval required',
    border: 'border-[color:var(--color-require-approval)]',
    dot: 'bg-[color:var(--color-require-approval)]',
    text: 'text-[color:var(--color-require-approval)]',
  },
  defer: {
    label: 'Deferred',
    border: 'border-muted-foreground',
    dot: 'bg-muted-foreground',
    text: 'text-muted-foreground',
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
  if (
    lower === 'permit' ||
    lower === 'deny' ||
    lower === 'transform' ||
    lower === 'require_approval' ||
    lower === 'defer'
  ) {
    return lower;
  }
  return 'unknown';
}
