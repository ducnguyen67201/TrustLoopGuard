'use client';

import {
  IconCheck,
  IconCircleDot,
  IconCopy,
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
import { InfoHint } from '@/components/ui/info-hint';
import { PageHeader } from '@/components/ui/page-header';
import { CreateApiKeyDialog } from '@/components/workspace/CreateApiKeyDialog';
import { useRowSelection } from '@/hooks/use-row-selection';
import { revokeApiKeys } from '@/lib/api-keys';
import type { ApiKeyRow, DashboardShellData } from '@/lib/server/dashboard-data';

type ApiKeysPageData = DashboardShellData & { apiKeys: ApiKeyRow[] };

function KeyStatusBadge({ status }: { status: string }) {
  if (status === 'Active') {
    return (
      <span className="inline-flex items-center gap-1.5">
        <Badge variant="outline" className="gap-1.5 text-xs">
          <IconCircleDot className="size-3 text-[var(--color-allow)]" aria-hidden />
          Active
        </Badge>
        <InfoHint label="What does Active mean?">
          Active means this key works right now — your app can use it to connect.
        </InfoHint>
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1.5">
      <Badge variant="secondary" className="text-xs text-muted-foreground">
        {status}
      </Badge>
      <InfoHint label="What does Revoked mean?">
        Revoked means this key is turned off and can no longer be used to connect.
      </InfoHint>
    </span>
  );
}

function CopyableId({ id, name, prefix }: { id: string; name: string; prefix: string }) {
  const [copied, setCopied] = useState(false);

  async function copyId() {
    try {
      // Display the truncated prefix, but copy the full key id so a value
      // pasted into a support ticket is complete and matchable.
      await navigator.clipboard.writeText(id);
      setCopied(true);
      toast.success('Key ID copied');
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error("Couldn't copy — select the ID and copy it manually.");
    }
  }

  return (
    <span className="inline-flex items-center gap-1">
      <span className="truncate font-mono text-xs text-muted-foreground">{prefix}…</span>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        onClick={copyId}
        aria-label={`Copy the ID for ${name}`}
      >
        {copied ? (
          <IconCheck className="text-[var(--color-allow)]" />
        ) : (
          <IconCopy />
        )}
      </Button>
    </span>
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
      header: 'Name',
      cell: (row) => (
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate text-sm font-medium text-foreground">{row.name}</span>
          <CopyableId id={row.id} name={row.name} prefix={row.prefix} />
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
      id: 'principal',
      header: 'Principal',
      cell: (row) =>
        row.principal === null ? (
          <span className="text-xs text-muted-foreground">Whole workspace</span>
        ) : (
          <span className="truncate font-mono text-xs">{row.principal}</span>
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
      cell: (row) =>
        row.lastUsed === 'Never' ? (
          <span className="text-xs text-muted-foreground">Not used yet</span>
        ) : (
          <span className="text-sm text-muted-foreground">{row.lastUsed}</span>
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
                Turn off key
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
      toast.success(
        ids.length === 1 ? 'Key turned off — it can no longer connect' : `${ids.length} keys turned off`,
      );
      router.refresh();
    } catch (err) {
      toast.error(
        err instanceof Error
          ? `Couldn't revoke: ${err.message}. Please try again.`
          : "Couldn't revoke the key. Please try again.",
      );
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
        help={<InfoHint term="apiKey" />}
        description="An API key is a secret token your app uses to connect to the guardrail. Create a key here, then paste it into your app. If a key is ever exposed, revoke it to turn it off instantly."
        actions={createDialog}
      />

      {hasKeys ? (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <SummaryTile label="All keys" value={apiKeys.length} />
          <SummaryTile label="Active" value={activeCount} accent="active" />
          <SummaryTile label="Turned off" value={revokedCount} accent="revoked" />
        </div>
      ) : null}

      <Card>
        <CardHeader className="flex flex-row items-center justify-between gap-3 space-y-0">
          <CardTitle className="flex items-center gap-2">
            <IconShieldLock className="size-4 text-muted-foreground" aria-hidden />
            Your keys
          </CardTitle>
          {hasKeys ? (
            <span className="text-xs tabular-nums text-muted-foreground">
              {activeCount} active of {apiKeys.length}
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
                    label: 'Turn off',
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
                caption="API keys for this workspace"
                empty="You don't have any API keys yet."
              />
            </>
          ) : (
            <EmptyState
              icon={<IconKey />}
              title="Create your first API key"
              description="A key is the secret token your app uses to connect to the guardrail. Create one, copy it once, and paste it into your app to start checking traffic."
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
              {revokeCount === 1 ? 'Turn off this API key?' : `Turn off ${revokeCount} API keys?`}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {revokeCount === 1 ? 'This key' : 'These keys'} will stop working right away, and any
              app still using {revokeCount === 1 ? 'it' : 'them'} will lose access. This can&apos;t be
              undone — you&apos;ll need to create a new key to reconnect.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep it active</AlertDialogCancel>
            <AlertDialogAction onClick={confirmRevoke}>Turn off key</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
