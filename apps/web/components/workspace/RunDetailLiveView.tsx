'use client';

import { Fragment, useCallback, useEffect, useMemo, useState } from 'react';
import { IconActivity, IconRefresh } from '@tabler/icons-react';

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
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
  parseRunDetailSnapshot,
  type RunDetailSnapshot,
} from '@/lib/run-detail-live';

const POLL_INTERVAL_MS = 1000;

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

  useEffect(() => {
    const interval = window.setInterval(() => {
      if (document.visibilityState === 'visible') {
        void refresh();
      }
    }, POLL_INTERVAL_MS);

    return () => window.clearInterval(interval);
  }, [refresh]);

  const tracesByEvent = useMemo(() => {
    const grouped = new Map<string, RunDetailSnapshot['traces']>();
    for (const trace of snapshot.traces) {
      if (!trace.runEventId) continue;
      const existing = grouped.get(trace.runEventId) ?? [];
      existing.push(trace);
      grouped.set(trace.runEventId, existing);
    }
    return grouped;
  }, [snapshot.traces]);

  const ungroupedTraces = useMemo(
    () => snapshot.traces.filter((trace) => !trace.runEventId),
    [snapshot.traces],
  );

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
          <Badge variant="outline" className="gap-1 rounded-sm">
            <IconActivity className="size-3.5 text-green-600" />
            Live
          </Badge>
          <span>Updated {relativeSync(lastSync)}</span>
          {error ? <span className="text-destructive">{error}</span> : null}
        </div>
        <Button
          variant="outline"
          size="sm"
          className="gap-2"
          onClick={() => void refresh()}
          disabled={isRefreshing}
        >
          <IconRefresh className={isRefreshing ? 'size-4 animate-spin' : 'size-4'} />
          Refresh
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Stat label="Traces" value={String(snapshot.run.traces)} />
        <Stat label="Blocked" value={String(snapshot.run.blocked)} />
        <Stat label="Escalated" value={String(snapshot.run.escalated)} />
        <Stat label="p95 latency" value={snapshot.run.latency} />
      </div>

      <Card>
        <CardHeader>
          <CardDescription>{snapshot.run.agent}</CardDescription>
          <CardTitle className="font-mono text-base">{snapshot.run.shortId}</CardTitle>
          <CardAction>
            <Badge variant="outline" className="rounded-sm">
              {snapshot.run.status}
            </Badge>
          </CardAction>
        </CardHeader>
        <CardContent className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <DetailItem label="Kind" value={snapshot.run.kind} />
          <DetailItem label="External ID" value={snapshot.run.externalId} />
          <DetailItem label="Started" value={snapshot.run.startedAt} />
          <DetailItem label="Ended" value={snapshot.run.endedAt} />
          <DetailItem label="Run ID" value={snapshot.run.id} className="md:col-span-2 lg:col-span-4" />
          {snapshot.run.metadata.map((item) => (
            <DetailItem key={item.label} label={item.label} value={item.value} />
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardDescription>Events and guardrail decisions refresh while this page is open</CardDescription>
          <CardTitle>Live timeline</CardTitle>
        </CardHeader>
        <CardContent>
          {snapshot.events.length === 0 && snapshot.traces.length === 0 ? (
            <TimelineEmptyState />
          ) : (
            <ScrollArea className="max-h-[52vh] pr-3">
              <div className="grid gap-3">
                {snapshot.events.map((event) => (
                  <RunEventTimelineItem
                    key={event.id}
                    event={event}
                    traces={tracesByEvent.get(event.id) ?? []}
                  />
                ))}
                {ungroupedTraces.length > 0 ? (
                  <section className="grid gap-2 border p-3">
                    <div className="font-medium">Guardrail checks</div>
                    <TraceList traces={ungroupedTraces} />
                  </section>
                ) : null}
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function TimelineEmptyState() {
  return (
    <div className="border p-4 text-sm text-muted-foreground">
      Waiting for events or traces on this run.
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <Card>
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle>{value}</CardTitle>
      </CardHeader>
    </Card>
  );
}

function DetailItem({
  label,
  value,
  className,
}: {
  label: string;
  value: string;
  className?: string;
}) {
  return (
    <div className={className}>
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 break-words text-sm">{value}</div>
    </div>
  );
}

function RunEventTimelineItem({
  event,
  traces,
}: {
  event: RunDetailSnapshot['events'][number];
  traces: RunDetailSnapshot['traces'];
}) {
  return (
    <section className="grid gap-3 border p-3 md:grid-cols-[7rem_1fr_auto]">
      <div>
        <div className="text-xs text-muted-foreground">#{event.sequence}</div>
        <Badge variant="outline" className="mt-1 rounded-sm">
          {event.kind}
        </Badge>
      </div>
      <div className="min-w-0">
        <div className="font-medium">{event.label}</div>
        <div className="mt-2 grid gap-2 text-sm md:grid-cols-2">
          <div>
            <div className="text-xs text-muted-foreground">Input</div>
            <div className="mt-1 break-words">{event.input}</div>
          </div>
          <div>
            <div className="text-xs text-muted-foreground">Output</div>
            <div className="mt-1 break-words">{event.output}</div>
          </div>
        </div>
        {event.metadata.length > 0 ? (
          <div className="mt-3 flex flex-wrap gap-x-4 gap-y-2 text-xs text-muted-foreground">
            {event.metadata.map((item) => (
              <span key={item.label}>
                {item.label}: <span className="text-foreground">{item.value}</span>
              </span>
            ))}
          </div>
        ) : null}
        {traces.length > 0 ? (
          <div className="mt-3">
            <TraceList traces={traces} />
          </div>
        ) : null}
      </div>
      <div className="text-sm text-muted-foreground md:text-right">
        <div>{event.time}</div>
      </div>
    </section>
  );
}

function TraceList({ traces }: { traces: RunDetailSnapshot['traces'] }) {
  return (
    <div className="border bg-muted/30">
      {traces.map((trace, index) => (
        <Fragment key={trace.id}>
          {index > 0 ? <Separator /> : null}
          <TraceItem trace={trace} />
        </Fragment>
      ))}
    </div>
  );
}

function TraceItem({ trace }: { trace: RunDetailSnapshot['traces'][number] }) {
  return (
    <div className="grid gap-2 p-3 text-sm md:grid-cols-[8rem_1fr_auto]">
      <div>
        <Badge variant="outline" className="rounded-sm">
          {trace.verdict}
        </Badge>
        <div className="mt-2 text-xs text-muted-foreground">{trace.phase}</div>
      </div>
      <div className="min-w-0">
        <div className="break-words font-medium">{trace.policy}</div>
        <div className="mt-1 break-words text-muted-foreground">{trace.reason}</div>
        {trace.safeOutput ? (
          <div className="mt-2 border bg-background p-2">
            <div className="text-xs text-muted-foreground">Safe output</div>
            <div className="mt-1 break-words">{trace.safeOutput}</div>
          </div>
        ) : null}
        <div className="mt-2 break-all font-mono text-xs text-muted-foreground">{trace.id}</div>
      </div>
      <div className="text-muted-foreground md:text-right">
        <div>{trace.latency}</div>
        <div className="mt-1 text-xs">{trace.time}</div>
      </div>
    </div>
  );
}

function relativeSync(date: Date): string {
  const seconds = Math.max(0, Math.round((Date.now() - date.getTime()) / 1000));
  if (seconds < 2) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  return `${Math.round(seconds / 60)}m ago`;
}
