'use client';

import Link from 'next/link';
import { IconArrowRight, IconBolt, IconRefresh, IconRobot } from '@tabler/icons-react';
import { useCallback, useState } from 'react';

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
import { Skeleton } from '@/components/ui/skeleton';
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
        <CardDescription>Grouped agent executions from SDK runtime checks</CardDescription>
        <CardTitle className="flex items-center gap-2">
          <IconBolt className="size-5 text-primary" />
          Recent runs
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
          <DataTable
            columns={runColumns}
            rows={runs}
            getRowKey={(run) => run.id}
            caption="Recent agent runs"
            empty="No runs recorded in this workspace yet."
          />
        ) : (
          <EmptyState
            icon={<IconRobot />}
            title="No runs recorded yet"
            description="A run groups every guardrail decision an agent makes during one SDK session. Connect an agent and its first run will stream in here."
            action={
              <Button asChild size="sm" variant="outline">
                <Link href="/api-keys">Connect an agent</Link>
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
  const stats: ReadonlyArray<{ label: string; value: number; tone: 'neutral' | 'block' | 'escalate' }> =
    [
      { label: 'Runs', value: totals.runs, tone: 'neutral' },
      { label: 'Traces', value: totals.traces, tone: 'neutral' },
      { label: 'Blocked', value: totals.blocked, tone: 'block' },
      { label: 'Escalated', value: totals.escalated, tone: 'escalate' },
    ];

  return (
    <dl className="grid grid-cols-2 divide-x divide-y divide-border border-b sm:grid-cols-4 sm:divide-y-0">
      {stats.map((stat) => (
        <div key={stat.label} className="flex flex-col gap-1 px-5 py-4">
          <dt className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
            {stat.label}
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
    <div className="grid gap-3" aria-hidden="true">
      <Skeleton className="h-9 w-full" />
      {Array.from({ length: 6 }).map((_, index) => (
        <Skeleton key={index} className="h-11 w-full" />
      ))}
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

const runColumns: DataTableColumn<RunRow>[] = [
  {
    id: 'id',
    header: 'Run',
    cell: (row) => (
      <Link
        className="font-mono text-xs text-foreground underline-offset-4 hover:underline"
        href={row.href}
      >
        {row.shortId}
      </Link>
    ),
  },
  { id: 'agent', header: 'Agent', cell: (row) => row.agent },
  { id: 'environment', header: 'Environment', cell: (row) => row.environment },
  { id: 'kind', header: 'Kind', cell: (row) => row.kind },
  {
    id: 'status',
    header: 'Status',
    cell: (row) => (
      <Badge variant="secondary" className="font-mono text-[0.7rem]">
        {row.status}
      </Badge>
    ),
  },
  {
    id: 'externalId',
    header: 'External ID',
    cell: (row) => row.externalId,
    cellClassName: 'font-mono text-xs text-muted-foreground',
  },
  {
    id: 'traces',
    header: 'Traces',
    cell: (row) => row.traces,
    align: 'right',
    cellClassName: 'font-data',
  },
  {
    id: 'blocked',
    header: 'Blocked',
    cell: (row) => <CountCell value={row.blocked} tone="block" />,
    align: 'right',
  },
  {
    id: 'escalated',
    header: 'Escalated',
    cell: (row) => <CountCell value={row.escalated} tone="escalate" />,
    align: 'right',
  },
  {
    id: 'latency',
    header: 'p95',
    cell: (row) => row.latency,
    align: 'right',
    cellClassName: 'font-data text-xs text-muted-foreground',
  },
  {
    id: 'started',
    header: 'Started',
    cell: (row) => row.started,
    cellClassName: 'text-muted-foreground',
  },
  {
    id: 'open',
    header: '',
    cell: (row) => (
      <Button asChild variant="ghost" size="icon-sm">
        <Link href={row.href} aria-label={`Open run ${row.shortId}`}>
          <IconArrowRight />
        </Link>
      </Button>
    ),
    align: 'right',
  },
];
