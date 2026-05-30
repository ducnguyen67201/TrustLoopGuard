'use client';

import { IconKeyOff } from '@tabler/icons-react';
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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { CreateApiKeyDialog } from '@/components/workspace/CreateApiKeyDialog';
import { useRowSelection } from '@/hooks/use-row-selection';
import { revokeApiKeys } from '@/lib/api-keys';
import type { ApiKeyRow, DashboardShellData } from '@/lib/server/dashboard-data';

type ApiKeysPageData = DashboardShellData & { apiKeys: ApiKeyRow[] };

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

  const apiKeyColumns: DataTableColumn<ApiKeyRow>[] = [
    { id: 'name', header: 'Name', cell: (row) => row.name },
    { id: 'environment', header: 'Environment', cell: (row) => row.environment },
    {
      id: 'prefix',
      header: 'Prefix',
      cell: (row) => row.prefix,
      cellClassName: 'font-mono text-xs',
    },
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
      id: 'lastUsed',
      header: 'Last used',
      cell: (row) => row.lastUsed,
      cellClassName: 'text-muted-foreground',
    },
    { id: 'createdBy', header: 'Created by', cell: (row) => row.createdBy },
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

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">{data.activeWorkspace.name}</p>
          <h2 className="text-2xl font-semibold">API Keys</h2>
        </div>
        <CreateApiKeyDialog
          environments={data.environments}
          activeEnvironmentId={data.activeEnvironment.id}
        />
      </div>

      <Card>
        <CardHeader>
          <CardDescription>Workspace-scoped runtime credentials</CardDescription>
          <CardTitle>API keys</CardTitle>
        </CardHeader>
        <CardContent>
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
                onSelect: () => setRevokeTarget(activeSelectedApiKeys.map((apiKey) => apiKey.id)),
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
            empty="No API keys issued yet."
          />
        </CardContent>
      </Card>

      <AlertDialog
        open={revokeTarget !== null}
        onOpenChange={(open) => !open && setRevokeTarget(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Revoke API keys?</AlertDialogTitle>
            <AlertDialogDescription>
              This immediately prevents the selected runtime credentials from authenticating SDK
              requests.
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
