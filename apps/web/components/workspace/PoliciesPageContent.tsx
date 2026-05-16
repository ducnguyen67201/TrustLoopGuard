'use client';

import { IconDotsVertical, IconPlus, IconTrash } from '@tabler/icons-react';
import { Power, PowerOff } from 'lucide-react';
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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { PolicyCreateDialog } from '@/components/workspace/PolicyCreateDialog';
import { useRowSelection } from '@/hooks/use-row-selection';
import {
  deletePolicy,
  getPolicy,
  setPoliciesEnabled,
  setPolicyEnabled,
  upsertPolicy,
} from '@/lib/policies';
import type { AgentRow, DashboardShellData, PolicyRow } from '@/lib/server/dashboard-data';

type PoliciesPageData = DashboardShellData & { agents: AgentRow[]; policies: PolicyRow[] };

type DeleteTarget = { ids: string[]; label: string } | null;

export function PoliciesPageContent({ data }: { data: PoliciesPageData }) {
  const router = useRouter();
  const [policies, setPolicies] = useState(data.policies);
  const [busyIds, setBusyIds] = useState<string[]>([]);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget>(null);
  const [editor, setEditor] = useState<{
    open: boolean;
    policyId: string | null;
    sourceYaml: string;
  }>({ open: false, policyId: null, sourceYaml: '' });
  const [editorLoading, setEditorLoading] = useState(false);
  const [editorSaving, setEditorSaving] = useState(false);
  const { selectedIds, selectedIdSet, setSelectedIds, clearSelection } = useRowSelection();

  useEffect(() => {
    setPolicies(data.policies);
    clearSelection();
  }, [clearSelection, data.policies]);

  const busyIdSet = useMemo(() => new Set(busyIds), [busyIds]);
  const selectedPolicies = policies.filter((policy) => selectedIdSet.has(policy.id));

  const policyColumns: DataTableColumn<PolicyRow>[] = [
    {
      id: 'id',
      header: 'Policy',
      cell: (row) => row.id,
      cellClassName: 'font-mono text-xs',
    },
    {
      id: 'description',
      header: 'Description',
      cell: (row) => row.description,
      cellClassName: 'text-muted-foreground',
    },
    { id: 'agent', header: 'Agent', cell: (row) => row.agent },
    {
      id: 'severity',
      header: 'Severity',
      cell: (row) => (
        <Badge variant="outline" className="rounded-sm">
          {row.severity}
        </Badge>
      ),
    },
    { id: 'action', header: 'Action', cell: (row) => row.action },
    {
      id: 'enabled',
      header: 'Enabled',
      cell: (row) => (
        <Switch
          checked={row.enabled}
          disabled={busyIdSet.has(row.id)}
          onCheckedChange={(enabled) => void updateOneEnabled(row.id, enabled)}
          aria-label={`Toggle ${row.id}`}
        />
      ),
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cell: (row) => (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon-sm">
              <IconDotsVertical />
              <span className="sr-only">Actions</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={() => void openEditor(row.id)}>Edit YAML</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              variant="destructive"
              onSelect={() => setDeleteTarget({ ids: [row.id], label: row.id })}
            >
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      ),
    },
  ];

  async function updateOneEnabled(policyId: string, enabled: boolean) {
    setBusyIds((prev) => [...prev, policyId]);
    const previous = policies;
    setPolicies((prev) =>
      prev.map((policy) => (policy.id === policyId ? { ...policy, enabled } : policy)),
    );
    try {
      await setPolicyEnabled(policyId, enabled);
      toast.success(enabled ? 'Policy enabled' : 'Policy disabled');
      router.refresh();
    } catch (err) {
      setPolicies(previous);
      toast.error(describeError(err));
    } finally {
      setBusyIds((prev) => prev.filter((id) => id !== policyId));
    }
  }

  async function updateSelectedEnabled(enabled: boolean) {
    const ids = selectedPolicies.map((policy) => policy.id);
    if (ids.length === 0) return;
    setBusyIds((prev) => Array.from(new Set([...prev, ...ids])));
    const previous = policies;
    setPolicies((prev) =>
      prev.map((policy) => (ids.includes(policy.id) ? { ...policy, enabled } : policy)),
    );
    try {
      await setPoliciesEnabled(ids, enabled);
      clearSelection();
      toast.success(enabled ? 'Policies enabled' : 'Policies disabled');
      router.refresh();
    } catch (err) {
      setPolicies(previous);
      toast.error(describeError(err));
    } finally {
      setBusyIds((prev) => prev.filter((id) => !ids.includes(id)));
    }
  }

  async function openEditor(policyId: string) {
    setEditor({ open: true, policyId, sourceYaml: '' });
    setEditorLoading(true);
    try {
      const policy = await getPolicy(policyId);
      setEditor({ open: true, policyId, sourceYaml: policy.source_yaml });
    } catch (err) {
      toast.error(describeError(err));
      setEditor({ open: false, policyId: null, sourceYaml: '' });
    } finally {
      setEditorLoading(false);
    }
  }

  async function saveEditor() {
    if (editor.policyId === null) return;
    setEditorSaving(true);
    try {
      await upsertPolicy(editor.sourceYaml);
      toast.success('Policy updated');
      setEditor({ open: false, policyId: null, sourceYaml: '' });
      router.refresh();
    } catch (err) {
      toast.error(describeError(err));
    } finally {
      setEditorSaving(false);
    }
  }

  async function confirmDelete() {
    if (deleteTarget === null) return;
    const ids = deleteTarget.ids;
    setDeleteTarget(null);
    setBusyIds((prev) => Array.from(new Set([...prev, ...ids])));
    try {
      await Promise.all(ids.map((id) => deletePolicy(id)));
      setPolicies((prev) => prev.filter((policy) => !ids.includes(policy.id)));
      clearSelection();
      toast.success(ids.length === 1 ? 'Policy deleted' : 'Policies deleted');
      router.refresh();
    } catch (err) {
      toast.error(describeError(err));
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
          <h2 className="text-2xl font-semibold">Policies</h2>
        </div>
        <div className="flex flex-col gap-2 sm:flex-row">
          <PolicyCreateDialog agents={data.agents} workspaceSlug={data.activeWorkspace.slug}>
            <IconPlus />
            New policy
          </PolicyCreateDialog>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardDescription>Workspace-authored guardrails</CardDescription>
          <CardTitle>Policies</CardTitle>
        </CardHeader>
        <CardContent>
          <BatchActionBar
            selectedCount={selectedPolicies.length}
            onClear={clearSelection}
            actions={[
              {
                id: 'enable',
                label: 'Enable',
                icon: Power,
                onSelect: () => void updateSelectedEnabled(true),
              },
              {
                id: 'disable',
                label: 'Disable',
                icon: PowerOff,
                onSelect: () => void updateSelectedEnabled(false),
              },
              {
                id: 'delete',
                label: 'Delete',
                icon: IconTrash,
                variant: 'destructive',
                onSelect: () =>
                  setDeleteTarget({
                    ids: selectedPolicies.map((policy) => policy.id),
                    label: `${selectedPolicies.length} policies`,
                  }),
              },
            ]}
            className="mb-3"
          />
          <DataTable
            columns={policyColumns}
            rows={policies}
            getRowKey={(policy) => policy.id}
            selection={{
              selectedRowKeys: selectedIds,
              onSelectedRowKeysChange: setSelectedIds,
            }}
            empty="No policies authored yet."
          />
        </CardContent>
      </Card>

      <Dialog
        open={editor.open}
        onOpenChange={(open) =>
          setEditor((prev) => ({ ...prev, open, policyId: open ? prev.policyId : null }))
        }
      >
        <DialogContent className="max-w-4xl">
          <DialogHeader>
            <DialogTitle>Edit policy YAML</DialogTitle>
            <DialogDescription>
              Save the full policy document exactly as Rust will validate and store it.
            </DialogDescription>
          </DialogHeader>
          <Textarea
            value={editor.sourceYaml}
            onChange={(event) =>
              setEditor((prev) => ({ ...prev, sourceYaml: event.target.value }))
            }
            disabled={editorLoading || editorSaving}
            className="min-h-96 font-mono text-sm"
            placeholder={editorLoading ? 'Loading policy...' : undefined}
          />
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setEditor({ open: false, policyId: null, sourceYaml: '' })}
              disabled={editorSaving}
            >
              Cancel
            </Button>
            <Button
              type="button"
              onClick={() => void saveEditor()}
              disabled={editorLoading || editorSaving || editor.sourceYaml.trim() === ''}
            >
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog
        open={deleteTarget !== null}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete policies?</AlertDialogTitle>
            <AlertDialogDescription>
              This will remove {deleteTarget?.label} from the Rust policy store.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDelete}>Delete</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

function describeError(err: unknown): string {
  if (err instanceof Error) return err.message;
  return 'unknown error';
}
