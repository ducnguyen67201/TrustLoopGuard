'use client';

import Link from 'next/link';
import { IconArrowRight } from '@tabler/icons-react';
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

  return (
    <Card>
      <CardHeader>
        <CardDescription>Grouped agent executions from SDK runtime checks</CardDescription>
        <CardTitle>Recent runs</CardTitle>
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
      <CardContent>
        <DataTable
          columns={runColumns}
          rows={runs}
          getRowKey={(run) => run.id}
          empty="No runs recorded in this workspace yet."
        />
      </CardContent>
    </Card>
  );
}

const runColumns: DataTableColumn<RunRow>[] = [
  {
    id: 'id',
    header: 'Run',
    cell: (row) => (
      <Link className="font-mono text-xs underline-offset-4 hover:underline" href={row.href}>
        {row.shortId}
      </Link>
    ),
  },
  { id: 'agent', header: 'Agent', cell: (row) => row.agent },
  { id: 'kind', header: 'Kind', cell: (row) => row.kind },
  {
    id: 'status',
    header: 'Status',
    cell: (row) => (
      <Badge variant="outline" className="rounded-sm">
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
  { id: 'traces', header: 'Traces', cell: (row) => row.traces, align: 'right' },
  { id: 'blocked', header: 'Blocked', cell: (row) => row.blocked, align: 'right' },
  { id: 'escalated', header: 'Escalated', cell: (row) => row.escalated, align: 'right' },
  { id: 'latency', header: 'p95', cell: (row) => row.latency, align: 'right' },
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
