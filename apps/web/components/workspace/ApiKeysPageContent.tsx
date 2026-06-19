'use client';

import {
  IconCircleDot,
  IconDotsVertical,
  IconKey,
  IconKeyOff,
  IconShieldLock,
} from '@tabler/icons-react';
import { useEffect, useMemo, useState } from 'react';
import { useRouter } from 'next/navigation';
import { toast } from 'sonner';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { BatchActionBar } from '@/components/ui/batch-action-bar';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { EmptyState } from '@/components/ui/empty-state';
import { PageHeader } from '@/components/ui/page-header';
import { CreateApiKeyDialog } from '@/components/workspace/CreateApiKeyDialog';
import { useRowSelection } from '@/hooks/use-row-selection';
import { revokeApiKeys } from '@/lib/api-keys';
import type { ApiKeyRow, DashboardShellData } from '@/lib/server/dashboard-data';

type ApiKeysPageData = DashboardShellData & { apiKeys: ApiKeyRow[] };

function KeyStatusBadge({ status }: { status: string }) {
  if (status === 'Active') {
    return (
      <Badge
        variant="outline"
        className="gap-1.5 font-mono text-[0.6875rem] uppercase tracking-wide"
      >
        <IconCircleDot className="size-3 text-[var(--color-allow)]" aria-hidden />
        Active
      </Badge>
    );
  }
  return (
    <Badge
      variant="secondary"
      className="font-mono text-[0.6875rem] uppercase tracking-wide text-muted-foreground"
    >
      {status}
    </Badge>
  );
}

function SummaryTile({
  label,
  value,
  accent,
}: {
  label: string;
  value: number;
  accent?: 'active' | 'revoked';
}) {
  const valueClass =
    accent === 'active'
      ? 'text-[var(--color-allow)]'
      : accent === 'revoked'
        ? 'text-muted-foreground'
        : 'text-foreground';
  return (
    <div className="rounded-lg border bg-card px-4 py-3">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">{label}</p>
      <p className={`mt-1 font-mono text-2xl font-semibold tabular-nums ${valueClass}`}>{value}</p>
    </div>
  );
}

export function ApiKeysPageContent({ data }: { data: ApiKeysPageData }) {
  const router = useRouter();
  const [apiKeys, setApiKeys] = useState(data.apiKeys);
  const [busyIds, setBusyIds] = useState<string[]>([]);
  const [revokeTarget, setRevokeTarget] = useState<string[] | null>(null);
  const { selectedIds, selectedIdSet, setSelectedIds, clearSelection } = useRowSelection();

  useEffect(() => {
    setApiKeys(data.apiKeys);
    clearSelection();
  }, [clearSelection, data.apiKeys]);

  const busyIdSet = useMemo(() => new Set(busyIds), [busyIds]);
  const selectedApiKeys = apiKeys.filter((apiKey) => selectedIdSet.has(apiKey.id));
  const activeSelectedApiKeys = selectedApiKeys.filter((apiKey) => apiKey.status === 'Active');
  const activeCount = useMemo(
    () => apiKeys.filter((apiKey) => apiKey.status === 'Active').length,
    [apiKeys],
  );
  const revokedCount = apiKeys.length - activeCount;
  const hasKeys = apiKeys.length > 0;

  const createDialog = (
    <CreateApiKeyDialog
      environments={data.environments}
      activeEnvironmentId={data.activeEnvironment.id}
    />
  );

  const apiKeyColumns: DataTableColumn<ApiKeyRow>[] = [
    {
      id: 'name',
      header: 'Key',
      cell: (row) => (
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate text-sm font-medium text-foreground">{row.name}</span>
          <span className="truncate font-mono text-xs text-muted-foreground">{row.prefix}…</span>
        </div>
      ),
    },
    {
      id: 'environment',
      header: 'Environment',
      cell: (row) => (
        <Badge variant="outline" className="font-mono text-[0.6875rem]">
          {row.environment}
        </Badge>
      ),
    },
    {
      id: 'status',
      header: 'Status',
      cell: (row) => <KeyStatusBadge status={row.status} />,
    },
    {
      id: 'lastUsed',
      header: 'Last used',
      cell: (row) => (
        <span className="font-mono text-xs tabular-nums text-muted-foreground">{row.lastUsed}</span>
      ),
    },
    {
      id: 'createdBy',
      header: 'Created by',
      cell: (row) => <span className="text-sm">{row.createdBy}</span>,
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cell: (row) => {
        const canRevoke = row.status === 'Active' && !busyIdSet.has(row.id);
        return (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm">
                <IconDotsVertical />
                <span className="sr-only">Actions for {row.name}</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                variant="destructive"
                disabled={!canRevoke}
                onSelect={() => setRevokeTarget([row.id])}
              >
                <IconKeyOff />
                Revoke key
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        );
      },
    },
  ];

  async function confirmRevoke() {
    if (revokeTarget === null) return;
    const ids = revokeTarget;
    setRevokeTarget(null);
    setBusyIds((prev) => Array.from(new Set([...prev, ...ids])));
    try {
      await revokeApiKeys(ids);
      setApiKeys((prev) =>
        prev.map((apiKey) => (ids.includes(apiKey.id) ? { ...apiKey, status: 'Revoked' } : apiKey)),
      );
      clearSelection();
      toast.success(ids.length === 1 ? 'API key revoked' : 'API keys revoked');
      router.refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'unknown error');
      router.refresh();
    } finally {
      setBusyIds((prev) => prev.filter((id) => !ids.includes(id)));
    }
  }

  const revokeCount = revokeTarget?.length ?? 0;

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow={data.activeWorkspace.name}
        title="API keys"
        description="Workspace-scoped runtime credentials. SDKs present these as a bearer token on every check; revoke a key to cut off access immediately."
        actions={createDialog}
      />

      {hasKeys ? (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <SummaryTile label="Total" value={apiKeys.length} />
          <SummaryTile label="Active" value={activeCount} accent="active" />
          <SummaryTile label="Revoked" value={revokedCount} accent="revoked" />
        </div>
      ) : null}

      <Card>
        <CardHeader className="flex flex-row items-center justify-between gap-3 space-y-0">
          <CardTitle className="flex items-center gap-2">
            <IconShieldLock className="size-4 text-muted-foreground" aria-hidden />
            Issued keys
          </CardTitle>
          {hasKeys ? (
            <span className="font-mono text-xs tabular-nums text-muted-foreground">
              {activeCount} active / {apiKeys.length} total
            </span>
          ) : null}
        </CardHeader>
        <CardContent>
          {hasKeys ? (
            <>
              <BatchActionBar
                selectedCount={selectedApiKeys.length}
                onClear={clearSelection}
                actions={[
                  {
                    id: 'revoke',
                    label: 'Revoke',
                    icon: IconKeyOff,
                    variant: 'destructive',
                    disabled: activeSelectedApiKeys.length === 0,
                    onSelect: () =>
                      setRevokeTarget(activeSelectedApiKeys.map((apiKey) => apiKey.id)),
                  },
                ]}
                className="mb-3"
              />
              <DataTable
                columns={apiKeyColumns}
                rows={apiKeys}
                getRowKey={(apiKey) => apiKey.id}
                selection={{
                  selectedRowKeys: selectedIds,
                  onSelectedRowKeysChange: setSelectedIds,
                  getRowCanSelect: (apiKey) =>
                    apiKey.status === 'Active' && !busyIdSet.has(apiKey.id),
                }}
                caption="Runtime API keys for this workspace"
                empty="No API keys issued yet."
              />
            </>
          ) : (
            <EmptyState
              icon={<IconKey />}
              title="No API keys yet"
              description="Issue an environment-scoped key so your SDK integration can authenticate runtime checks against this workspace."
              action={createDialog}
            />
          )}
        </CardContent>
      </Card>

      <AlertDialog
        open={revokeTarget !== null}
        onOpenChange={(open) => !open && setRevokeTarget(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {revokeCount === 1 ? 'Revoke this API key?' : `Revoke ${revokeCount} API keys?`}
            </AlertDialogTitle>
            <AlertDialogDescription>
              This immediately prevents the selected runtime credentials from authenticating SDK
              requests. It cannot be undone — affected integrations must be issued a new key.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmRevoke}>Revoke</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
