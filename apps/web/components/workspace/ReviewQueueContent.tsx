'use client';

import { IconInbox, IconRefresh } from '@tabler/icons-react';
import Link from 'next/link';
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
import { type Verdict } from '@/components/workspace/ReviewActionDialog';
import { ReviewRowActions } from '@/components/workspace/ReviewRowActions';
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

type Filter = 'pending' | 'all' | 'escalate' | 'block' | 'reviewed';

const FILTERS: ReadonlyArray<{ value: Filter; label: string }> = [
  { value: 'pending', label: 'Pending' },
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

function traceHref(traceId: string, workspaceSlug: string): string {
  const query = workspaceSlug === '' ? '' : `?${new URLSearchParams({ workspace: workspaceSlug })}`;
  return `/traces/${encodeURIComponent(traceId)}${query}`;
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
  const [filter, setFilter] = useState<Filter>('pending');

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

  // Stamp the outcome on the row so it reads as reviewed without a refetch.
  // Used by both the one-click actions (optimistically, before the server
  // responds) and the dialog (after a confirmed success).
  const stampOutcome = useCallback((traceId: string, outcome: ReviewOutcome) => {
    setTraces((prev) =>
      prev.map((trace) =>
        trace.trace_id === traceId ? { ...trace, latest_review_outcome: outcome } : trace,
      ),
    );
  }, []);

  // Roll back when a one-click post fails. Inline actions only render on rows
  // whose latest_review_outcome is null (see the review column), so the prior
  // value is always null — restoring null is the correct rollback.
  const revertOutcome = useCallback((traceId: string) => {
    setTraces((prev) =>
      prev.map((trace) =>
        trace.trace_id === traceId ? { ...trace, latest_review_outcome: null } : trace,
      ),
    );
  }, []);

  const actionable = useMemo(
    () => traces.filter((trace) => isActionableVerdict(trace.decision)),
    [traces],
  );
  const rows = useMemo(
    () =>
      actionable.filter((trace) => {
        if (filter === 'all') return true;
        if (filter === 'pending') return trace.latest_review_outcome === null;
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
        cell: (row) => (
          <div className="grid gap-1">
            <span>{reasonOf(row.payload) ?? '—'}</span>
            <Link
              href={traceHref(row.trace_id, workspaceSlug)}
              className="text-xs font-medium text-foreground underline decoration-muted-foreground/40 underline-offset-4 transition-colors hover:decoration-foreground"
            >
              View trace
            </Link>
          </div>
        ),
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
            <Badge variant="outline" className="font-normal">
              {OUTCOME_LABEL[row.latest_review_outcome]}
            </Badge>
          ) : isActionableVerdict(row.decision) ? (
            <ReviewRowActions
              traceId={row.trace_id}
              verdict={row.decision}
              reason={reasonOf(row.payload)}
              workspaceSlug={workspaceSlug}
              onRecorded={stampOutcome}
              onRevert={revertOutcome}
            />
          ) : null,
      },
    ],
    [workspaceSlug, stampOutcome, revertOutcome],
  );

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
