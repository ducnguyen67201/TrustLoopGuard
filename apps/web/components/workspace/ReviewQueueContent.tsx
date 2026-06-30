'use client';

import { IconInbox, IconRefresh } from '@tabler/icons-react';
import { useCallback, useEffect, useMemo, useState } from 'react';

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
import { ReviewActionDialog, type Verdict } from '@/components/workspace/ReviewActionDialog';
import type { ReviewOutcome } from '@/lib/review-outcomes';

// Row source: GET /api/traces (recent, ≤100). We keep only the verdicts a human
// can act on. payload is arbitrary decision JSON — read defensively.
interface TraceRow {
  trace_id: string;
  decision: string;
  domain: string;
  created_at: string;
  latest_review_outcome: ReviewOutcome | null;
  payload: Record<string, unknown>;
}

interface TraceListResponse {
  traces: TraceRow[];
}

type Filter = 'all' | 'escalate' | 'block' | 'reviewed';

const FILTERS: ReadonlyArray<{ value: Filter; label: string }> = [
  { value: 'all', label: 'All' },
  { value: 'escalate', label: 'Escalated' },
  { value: 'block', label: 'Blocked' },
  { value: 'reviewed', label: 'Reviewed' },
];

const OUTCOME_LABEL: Record<ReviewOutcome, string> = {
  accepted: 'Accepted',
  corrected: 'Corrected',
  rejected: 'Rejected',
  false_positive: 'False positive',
  missed_issue: 'Missed issue',
  ignored: 'Ignored',
};

function isActionableVerdict(decision: string): decision is Verdict {
  return decision === 'escalate' || decision === 'block';
}

function reasonOf(payload: Record<string, unknown>): string | undefined {
  const reason = payload['reason'];
  return typeof reason === 'string' && reason !== '' ? reason : undefined;
}

// ponytail: relative time is computed at render; the Refresh button is the
// refresh path, so no auto-tick timer — labels go stale until the next load.
function relativeTime(iso: string): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return '—';
  const seconds = Math.round((Date.now() - then) / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}

export function ReviewQueueContent({ workspaceSlug }: { workspaceSlug: string }) {
  const [traces, setTraces] = useState<TraceRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>('all');

  const load = useCallback(
    async (signal?: AbortSignal) => {
      setLoading(true);
      setError(null);
      try {
        const query = new URLSearchParams({ limit: '100' });
        if (workspaceSlug !== '') query.set('workspace', workspaceSlug);
        const res = await fetch(`/api/traces?${query.toString()}`, {
          cache: 'no-store',
          signal: signal ?? null,
        });
        if (!res.ok) {
          throw new Error(`failed to load traces (${res.status})`);
        }
        const body = (await res.json()) as TraceListResponse;
        setTraces(Array.isArray(body.traces) ? body.traces : []);
      } catch (err) {
        // A newer load (workspace switch) aborted this one — drop it silently
        // so its stale result can't overwrite the current state.
        if (signal?.aborted === true) return;
        setError(err instanceof Error ? err.message : 'failed to load traces');
      } finally {
        if (signal?.aborted !== true) setLoading(false);
      }
    },
    [workspaceSlug],
  );

  useEffect(() => {
    const controller = new AbortController();
    void load(controller.signal);
    return () => controller.abort();
  }, [load]);

  const actionable = useMemo(
    () => traces.filter((trace) => isActionableVerdict(trace.decision)),
    [traces],
  );
  const rows = useMemo(
    () =>
      actionable.filter((trace) => {
        if (filter === 'all') return true;
        if (filter === 'reviewed') return trace.latest_review_outcome !== null;
        return trace.decision === filter;
      }),
    [actionable, filter],
  );

  const columns = useMemo<DataTableColumn<TraceRow>[]>(
    () => [
    {
      id: 'verdict',
      header: 'Verdict',
      cell: (row) =>
        isActionableVerdict(row.decision) ? (
          <Badge variant={row.decision}>{row.decision}</Badge>
        ) : (
          row.decision
        ),
    },
    { id: 'domain', header: 'Domain', cell: (row) => row.domain },
    {
      id: 'reason',
      header: 'Reason',
      cell: (row) => reasonOf(row.payload) ?? '—',
      cellClassName: 'max-w-md text-xs text-muted-foreground',
    },
    {
      id: 'when',
      header: 'When',
      cell: (row) => relativeTime(row.created_at),
      cellClassName: 'text-xs text-muted-foreground whitespace-nowrap',
    },
    {
      id: 'review',
      header: 'Review',
      align: 'right',
      cell: (row) =>
        row.latest_review_outcome !== null ? (
          <span className="text-sm text-muted-foreground">
            {OUTCOME_LABEL[row.latest_review_outcome]}
          </span>
        ) : isActionableVerdict(row.decision) ? (
          <ReviewActionDialog
            traceId={row.trace_id}
            verdict={row.decision}
            reason={reasonOf(row.payload)}
            workspaceSlug={workspaceSlug}
            onReviewed={load}
          />
        ) : null,
    },
  ], [workspaceSlug, load]);

  return (
    <Card className="overflow-hidden">
      <CardHeader className="border-b pb-6">
        <CardDescription>
          Agent actions the guard stopped — escalated for sign-off or blocked outright. Recording a
          decision logs an audit trail; it does not resume the action.
        </CardDescription>
        <CardTitle className="flex items-center gap-2">
          <IconInbox className="size-5 text-primary" />
          Review queue
        </CardTitle>
        <CardAction>
          <Button
            size="sm"
            variant="outline"
            onClick={() => void load()}
            disabled={loading}
            aria-label="Refresh"
          >
            <IconRefresh className={loading ? 'animate-spin motion-reduce:animate-none' : ''} />
            Refresh
          </Button>
        </CardAction>
      </CardHeader>

      <div
        role="group"
        aria-label="Filter review queue"
        className="flex flex-wrap gap-2 border-b px-5 py-3"
      >
        {FILTERS.map((option) => (
          <Button
            key={option.value}
            size="sm"
            variant={filter === option.value ? 'secondary' : 'ghost'}
            aria-pressed={filter === option.value}
            onClick={() => setFilter(option.value)}
          >
            {option.label}
          </Button>
        ))}
      </div>

      <CardContent className="pt-6">
        {error !== null ? (
          <EmptyState
            className="border-destructive/30 bg-destructive/5"
            icon={<IconRefresh />}
            title="Couldn't load the review queue"
            description={error}
            action={
              <Button size="sm" variant="outline" onClick={() => void load()} disabled={loading}>
                Try again
              </Button>
            }
          />
        ) : rows.length > 0 ? (
          <DataTable
            columns={columns}
            rows={rows}
            getRowKey={(row) => row.trace_id}
            caption="Escalated and blocked agent actions awaiting a human decision."
          />
        ) : (
          <EmptyState
            icon={<IconInbox />}
            title="Nothing waiting for review"
            description="When the guard escalates or blocks an agent action, it shows up here for a person to weigh in."
          />
        )}
      </CardContent>
    </Card>
  );
}
