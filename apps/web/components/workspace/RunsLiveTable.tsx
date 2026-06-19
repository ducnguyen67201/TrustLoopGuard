'use client';

import Link from 'next/link';
import {
  IconArrowRight,
  IconBolt,
  IconCheck,
  IconCopy,
  IconRefresh,
  IconRobot,
} from '@tabler/icons-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

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
import { InfoHint } from '@/components/ui/info-hint';
import { Skeleton } from '@/components/ui/skeleton';
import { VerdictLegend } from '@/components/ui/verdict-legend';
import {
  RefreshControls,
  useAutoRefresh,
  type RefreshMode,
} from '@/components/workspace/RefreshControls';
import type { RunRow } from '@/lib/server/dashboard-data';
import { parseRunsSnapshot } from '@/lib/runs-live';

export function RunsLiveTable({
  initialRuns,
  workspaceSlug,
}: {
  initialRuns: RunRow[];
  workspaceSlug: string;
}) {
  const [runs, setRuns] = useState(initialRuns);
  const [lastSync, setLastSync] = useState<Date>(() => new Date());
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<RefreshMode>('live');

  const refresh = useCallback(async () => {
    const params = new URLSearchParams({ workspace: workspaceSlug });
    setIsRefreshing(true);

    try {
      const response = await fetch(`/api/runs?${params.toString()}`, { cache: 'no-store' });
      if (!response.ok) {
        throw new Error(`runs refresh failed with ${response.status}`);
      }

      setRuns(parseRunsSnapshot(await response.json(), workspaceSlug));
      setLastSync(new Date());
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'runs refresh failed');
    } finally {
      setIsRefreshing(false);
    }
  }, [workspaceSlug]);

  useAutoRefresh(refresh, mode);

  const totals = aggregateRuns(runs);
  const hasRuns = runs.length > 0;
  // First paint with no seed data and a refresh already in flight: show structure,
  // not a bare empty card, so the surface never looks frozen on initial load.
  const showSkeleton = !hasRuns && isRefreshing && error === null;

  return (
    <Card className="overflow-hidden">
      <CardHeader className="border-b pb-6">
        <CardDescription>
          A live feed of every request the guardrail checked, newest first.
        </CardDescription>
        <CardTitle className="flex items-center gap-2">
          <IconBolt className="size-5 text-primary" />
          Recent activity
        </CardTitle>
        <CardAction>
          <RefreshControls
            mode={mode}
            onModeChange={setMode}
            onRefresh={() => void refresh()}
            isRefreshing={isRefreshing}
            lastSync={lastSync}
            error={error}
          />
        </CardAction>
      </CardHeader>

      {hasRuns ? <RunsSummary totals={totals} /> : null}

      <CardContent className="pt-6">
        {error && !hasRuns ? (
          <RunsErrorState message={error} onRetry={() => void refresh()} isRetrying={isRefreshing} />
        ) : showSkeleton ? (
          <RunsTableSkeleton />
        ) : hasRuns ? (
          <div className="grid gap-5">
            <DataTable
              columns={runColumns}
              rows={runs}
              getRowKey={(run) => run.id}
              caption="A live feed of every request the guardrail checked, newest first."
              empty="Nothing yet — requests appear here the moment your agents start calling the guardrail."
            />
            <div className="rounded-lg border bg-muted/20 px-4 py-3">
              <p className="mb-2.5 text-xs font-medium text-muted-foreground">
                What the colored counts mean
              </p>
              <VerdictLegend />
            </div>
          </div>
        ) : (
          <EmptyState
            icon={<IconRobot />}
            title="Nothing yet"
            description="Requests appear here the moment your agents start calling the guardrail. Connect an agent to see its first check stream in."
            action={
              <Button asChild size="sm" variant="outline">
                <Link href={`/api-keys?workspace=${encodeURIComponent(workspaceSlug)}`}>
                  Connect an agent
                </Link>
              </Button>
            }
          />
        )}
      </CardContent>
    </Card>
  );
}

type RunTotals = {
  runs: number;
  traces: number;
  blocked: number;
  escalated: number;
};

function aggregateRuns(runs: RunRow[]): RunTotals {
  return runs.reduce<RunTotals>(
    (acc, run) => ({
      runs: acc.runs + 1,
      traces: acc.traces + run.traces,
      blocked: acc.blocked + run.blocked,
      escalated: acc.escalated + run.escalated,
    }),
    { runs: 0, traces: 0, blocked: 0, escalated: 0 },
  );
}

/** Live aggregate strip — turns the raw table into a scannable signal at a glance. */
function RunsSummary({ totals }: { totals: RunTotals }) {
  const stats: ReadonlyArray<{
    label: string;
    value: number;
    tone: 'neutral' | 'block' | 'escalate';
    hint: string;
  }> = [
    { label: 'Requests', value: totals.runs, tone: 'neutral', hint: 'Requests shown below — every one your agents sent through the guardrail.' },
    { label: 'Checks', value: totals.traces, tone: 'neutral', hint: 'Total times the guardrail looked at a request, across all the requests below.' },
    { label: 'Blocked', value: totals.blocked, tone: 'block', hint: 'Requests stopped because they broke one of your rules.' },
    { label: 'Escalated', value: totals.escalated, tone: 'escalate', hint: 'Requests held for a person to review before continuing.' },
  ];

  return (
    <dl className="grid grid-cols-2 divide-x divide-y divide-border border-b sm:grid-cols-4 sm:divide-y-0">
      {stats.map((stat) => (
        <div key={stat.label} className="flex flex-col gap-1 px-5 py-4">
          <dt className="flex items-center gap-1 text-xs font-medium tracking-wide text-muted-foreground uppercase">
            {stat.label}
            <InfoHint label={`What does “${stat.label}” mean?`}>{stat.hint}</InfoHint>
          </dt>
          <dd
            className="font-data text-2xl"
            style={
              stat.tone === 'neutral' || stat.value === 0
                ? undefined
                : { color: `var(--color-${stat.tone})` }
            }
          >
            {stat.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}

function RunsTableSkeleton() {
  return (
    <div className="grid gap-3">
      <p
        className="flex items-center gap-2 text-sm text-muted-foreground"
        role="status"
        aria-live="polite"
      >
        <span className="relative flex size-2">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-current opacity-60 motion-reduce:hidden" />
          <span className="relative inline-flex size-2 rounded-full bg-current" />
        </span>
        Listening for incoming requests…
      </p>
      <div className="grid gap-3" aria-hidden="true">
        <Skeleton className="h-9 w-full" />
        {Array.from({ length: 6 }).map((_, index) => (
          <Skeleton key={index} className="h-11 w-full" />
        ))}
      </div>
    </div>
  );
}

function RunsErrorState({
  message,
  onRetry,
  isRetrying,
}: {
  message: string;
  onRetry: () => void;
  isRetrying: boolean;
}) {
  return (
    <EmptyState
      className="border-destructive/30 bg-destructive/5"
      icon={<IconRefresh />}
      title="Couldn't load runs"
      description={message}
      action={
        <Button size="sm" variant="outline" onClick={onRetry} disabled={isRetrying}>
          <IconRefresh className={isRetrying ? 'animate-spin motion-reduce:animate-none' : ''} />
          Try again
        </Button>
      }
    />
  );
}

/** Counts that carry verdict meaning get the verdict color; zero stays muted. */
function CountCell({ value, tone }: { value: number; tone: 'block' | 'escalate' }) {
  if (value === 0) {
    return <span className="font-data text-muted-foreground">0</span>;
  }
  return (
    <span className="font-data" style={{ color: `var(--color-${tone})` }}>
      {value}
    </span>
  );
}

/**
 * The "Request" cell: a short, human-readable label that links to the full
 * record, a one-tap copy for the full ID, and the relative time underneath — so
 * a non-technical reader never meets a bare UUID with no context.
 */
function RunIdCell({ row }: { row: RunRow }) {
  const [copied, setCopied] = useState(false);
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (resetTimer.current) clearTimeout(resetTimer.current);
    },
    [],
  );

  async function copyId() {
    try {
      await navigator.clipboard.writeText(row.id);
    } catch {
      toast.error("Couldn't copy — select the text and copy it manually.");
      return;
    }
    setCopied(true);
    if (resetTimer.current) clearTimeout(resetTimer.current);
    resetTimer.current = setTimeout(() => setCopied(false), 1600);
  }

  return (
    <div className="flex flex-col gap-0.5">
      <div className="flex items-center gap-1.5">
        <Link
          className="font-mono text-xs text-foreground underline-offset-4 hover:underline"
          href={row.href}
          title={row.id}
        >
          {row.shortId}
        </Link>
        <button
          type="button"
          onClick={copyId}
          aria-label={copied ? 'Request ID copied' : `Copy full request ID ${row.id}`}
          className="inline-flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground focus-visible:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 [&_svg]:size-3.5"
        >
          {copied ? <IconCheck className="text-[var(--color-allow)]" /> : <IconCopy />}
        </button>
      </div>
      <span className="text-[11px] text-muted-foreground">{row.started}</span>
    </div>
  );
}

/** A header label paired with a plain-language "what is this?" hint. */
function ColumnHead({ label, hint }: { label: string; hint: string }) {
  return (
    <span className="inline-flex items-center gap-1">
      {label}
      <InfoHint label={`What does “${label}” mean?`}>{hint}</InfoHint>
    </span>
  );
}

const runColumns: DataTableColumn<RunRow>[] = [
  {
    id: 'id',
    header: (
      <ColumnHead
        label="Request"
        hint="A single request your agent sent through the guardrail. Click it to see exactly how it was checked. The time below is when it started."
      />
    ),
    cell: (row) => <RunIdCell row={row} />,
  },
  { id: 'agent', header: 'Agent', cell: (row) => row.agent },
  {
    id: 'environment',
    header: (
      <ColumnHead
        label="Environment"
        hint="Which space this request belongs to — like testing or live traffic — so you can tell real activity apart from trials."
      />
    ),
    cell: (row) => row.environment,
  },
  {
    id: 'kind',
    header: (
      <ColumnHead label="Type" hint="What kind of request this was — for example, a chat session or a one-off check." />
    ),
    cell: (row) => row.kind,
  },
  {
    id: 'status',
    header: (
      <ColumnHead label="State" hint="Whether this request is still running or has finished." />
    ),
    cell: (row) => (
      <Badge variant="secondary" className="font-mono text-[0.7rem]">
        {row.status}
      </Badge>
    ),
  },
  {
    id: 'externalId',
    header: (
      <ColumnHead
        label="Your label"
        hint="A reference your own app attached to this request, so you can match it back to your system. Shows “None” if you didn’t set one."
      />
    ),
    cell: (row) => row.externalId,
    cellClassName: 'font-mono text-xs text-muted-foreground',
  },
  {
    id: 'traces',
    header: (
      <ColumnHead label="Checks" hint="How many times the guardrail looked at this request — once per input and output it reviewed." />
    ),
    cell: (row) => row.traces,
    align: 'right',
    cellClassName: 'font-data',
  },
  {
    id: 'blocked',
    header: (
      <ColumnHead label="Blocked" hint="Stopped — this request broke one of your rules. See the legend below for the colors." />
    ),
    cell: (row) => <CountCell value={row.blocked} tone="block" />,
    align: 'right',
  },
  {
    id: 'escalated',
    header: (
      <ColumnHead label="Escalated" hint="Held for a person to review before it continued. See the legend below for the colors." />
    ),
    cell: (row) => <CountCell value={row.escalated} tone="escalate" />,
    align: 'right',
  },
  {
    id: 'latency',
    header: (
      <ColumnHead label="Speed" hint="How long the guardrail took to check this request, in milliseconds. Lower is faster." />
    ),
    cell: (row) => row.latency,
    align: 'right',
    cellClassName: 'font-data text-xs text-muted-foreground',
  },
  {
    id: 'open',
    header: '',
    cell: (row) => (
      <Button asChild variant="ghost" size="icon-sm">
        <Link href={row.href} aria-label={`Open request ${row.shortId}`}>
          <IconArrowRight />
        </Link>
      </Button>
    ),
    align: 'right',
  },
];
