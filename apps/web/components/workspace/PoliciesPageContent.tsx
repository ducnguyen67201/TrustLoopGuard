'use client';

import { IconDotsVertical, IconPlus, IconTrash } from '@tabler/icons-react';
import { Power, PowerOff, ShieldCheck } from 'lucide-react';
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
import { EmptyState } from '@/components/ui/empty-state';
import { PageHeader } from '@/components/ui/page-header';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { PolicyBuilderEditor } from '@/components/policies/PolicyBuilderEditor';
import { PolicyYamlDiffEditor } from '@/components/policies/PolicyYamlDiffEditor';
import type { VersionEntry } from '@/components/policies/VersionPicker';
import { PolicyCreateDialog } from '@/components/workspace/PolicyCreateDialog';
import { PolicySeverityBadge } from '@/components/workspace/PolicySeverityBadge';
import { useRowSelection } from '@/hooks/use-row-selection';
import {
  aiEditPolicy,
  deletePolicy,
  getPolicy,
  getPolicyVersion,
  listPolicyVersions,
  setPoliciesEnabled,
  setPolicyEnabled,
  upsertPolicy,
} from '@/lib/policies';
import type { AgentRow, DashboardShellData, PolicyRow } from '@/lib/server/dashboard-data';

type PoliciesPageData = DashboardShellData & { agents: AgentRow[]; policies: PolicyRow[] };

type DeleteTarget = { ids: string[]; label: string } | null;

type VerdictVariant = 'allow' | 'rewrite' | 'block' | 'escalate';

const VERDICT_ACTIONS: ReadonlySet<string> = new Set(['allow', 'rewrite', 'block', 'escalate']);

function ActionBadge({ action }: { action: string }) {
  const key = action.toLowerCase();
  if (VERDICT_ACTIONS.has(key)) {
    return (
      <Badge variant={key as VerdictVariant} className="font-mono text-[0.6875rem]">
        {action}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="font-mono text-[0.6875rem]">
      {action}
    </Badge>
  );
}

export function PoliciesPageContent({ data }: { data: PoliciesPageData }) {
  const router = useRouter();
  const [policies, setPolicies] = useState(data.policies);
  const [busyIds, setBusyIds] = useState<string[]>([]);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget>(null);

  // Editor state
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorPolicyId, setEditorPolicyId] = useState<string | null>(null);
  const [editorOriginal, setEditorOriginal] = useState('');
  const [editorModified, setEditorModified] = useState('');
  const [editorLoading, setEditorLoading] = useState(false);
  const [editorSaving, setEditorSaving] = useState(false);

  // Version history state
  const [versions, setVersions] = useState<VersionEntry[]>([]);
  const [versionsLoading, setVersionsLoading] = useState(false);
  const [selectedVersion, setSelectedVersion] = useState<number | null>(null);

  const { selectedIds, selectedIdSet, setSelectedIds, clearSelection } = useRowSelection();

  useEffect(() => {
    setPolicies(data.policies);
    clearSelection();
  }, [clearSelection, data.policies]);

  const busyIdSet = useMemo(() => new Set(busyIds), [busyIds]);
  const selectedPolicies = policies.filter((policy) => selectedIdSet.has(policy.id));
  const enabledCount = useMemo(
    () => policies.filter((policy) => policy.enabled).length,
    [policies],
  );

  const policyColumns: DataTableColumn<PolicyRow>[] = [
    {
      id: 'id',
      header: 'Policy',
      cell: (row) => (
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate font-mono text-xs text-foreground">{row.id}</span>
          <span className="truncate text-xs text-muted-foreground">{row.description}</span>
        </div>
      ),
    },
    {
      id: 'agent',
      header: 'Agent',
      cell: (row) => <span className="text-sm">{row.agent}</span>,
    },
    {
      id: 'severity',
      header: 'Severity',
      cell: (row) => <PolicySeverityBadge severity={row.severity} />,
    },
    {
      id: 'action',
      header: 'Action',
      cell: (row) => <ActionBadge action={row.action} />,
    },
    {
      id: 'enabled',
      header: 'Status',
      align: 'right',
      cell: (row) => (
        <div className="flex items-center justify-end gap-2">
          <span
            className={
              row.enabled
                ? 'font-mono text-[0.6875rem] uppercase tracking-wide text-muted-foreground'
                : 'font-mono text-[0.6875rem] uppercase tracking-wide text-muted-foreground/60'
            }
          >
            {row.enabled ? 'Live' : 'Off'}
          </span>
          <Switch
            checked={row.enabled}
            disabled={busyIdSet.has(row.id)}
            onCheckedChange={(enabled) => void updateOneEnabled(row.id, enabled)}
            aria-label={`Toggle ${row.id}`}
          />
        </div>
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
    setEditorOpen(true);
    setEditorPolicyId(policyId);
    setEditorOriginal('');
    setEditorModified('');
    setVersions([]);
    setSelectedVersion(null);
    setVersionsLoading(true);
    setEditorLoading(true);
    try {
      const [policy, { versions: vs }] = await Promise.all([
        getPolicy(policyId),
        listPolicyVersions(policyId),
      ]);
      const yaml = policy.source_yaml;
      setEditorOriginal(yaml);
      setEditorModified(yaml);
      setVersions(vs);
      setSelectedVersion(vs[0]?.version ?? null);
    } catch (err) {
      toast.error(describeError(err));
      setEditorOpen(false);
    } finally {
      setEditorLoading(false);
      setVersionsLoading(false);
    }
  }

  async function handleVersionSelect(version: number) {
    if (editorPolicyId === null) return;
    setSelectedVersion(version);
    try {
      const detail = await getPolicyVersion(editorPolicyId, version);
      setEditorOriginal(detail.content);
    } catch (err) {
      toast.error(describeError(err));
    }
  }

  async function handleAiEdit(instruction: string) {
    try {
      const result = await aiEditPolicy(editorModified, instruction);
      setEditorModified(result);
    } catch (err) {
      toast.error(describeError(err));
    }
  }

  async function saveEditor() {
    if (editorPolicyId === null) return;
    setEditorSaving(true);
    try {
      await upsertPolicy(editorModified);
      toast.success('Policy updated');
      setEditorOpen(false);
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

  const hasPolicies = policies.length > 0;

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow={data.activeWorkspace.name}
        title="Policies"
        description={`Workspace-authored guardrails. Enablement applies to ${data.activeEnvironment.name}.`}
        actions={
          <PolicyCreateDialog agents={data.agents} workspaceSlug={data.activeWorkspace.slug}>
            <IconPlus />
            New policy
          </PolicyCreateDialog>
        }
      />

      <Card>
        <CardHeader className="flex flex-row items-center justify-between gap-3 space-y-0">
          <CardTitle>Deployed guardrails</CardTitle>
          {hasPolicies ? (
            <span className="font-mono text-xs tabular-nums text-muted-foreground">
              {enabledCount} live / {policies.length} total
            </span>
          ) : null}
        </CardHeader>
        <CardContent>
          {hasPolicies ? (
            <>
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
                caption="Workspace policies deployed to this environment"
                empty="No policies authored yet."
              />
            </>
          ) : (
            <EmptyState
              icon={<ShieldCheck />}
              title="No policies yet"
              description={`Author a guardrail to start enforcing rules in ${data.activeEnvironment.name}. Policies you create here deploy per environment.`}
              action={
                <PolicyCreateDialog agents={data.agents} workspaceSlug={data.activeWorkspace.slug}>
                  <IconPlus />
                  New policy
                </PolicyCreateDialog>
              }
            />
          )}
        </CardContent>
      </Card>

      {/* YAML Diff Editor with version picker + AI edit bar */}
      <Dialog open={editorOpen} onOpenChange={(open) => { if (!open) setEditorOpen(false); }}>
        <DialogContent className="max-h-[calc(100vh-2rem)] w-[calc(100vw-2rem)] overflow-y-auto sm:max-w-5xl">
          <DialogHeader>
            <DialogTitle className="font-mono">
              {editorPolicyId ?? 'Edit policy'}
            </DialogTitle>
            <DialogDescription>
              Use the builder for common policies or switch to YAML for advanced edits.
            </DialogDescription>
          </DialogHeader>

          <Tabs defaultValue="builder">
            <TabsList>
              <TabsTrigger value="builder">Builder</TabsTrigger>
              <TabsTrigger value="yaml">YAML</TabsTrigger>
            </TabsList>
            <TabsContent value="builder">
              <PolicyBuilderEditor
                yaml={editorModified}
                onYamlChange={setEditorModified}
                disabled={editorLoading || editorSaving}
              />
            </TabsContent>
            <TabsContent value="yaml">
              <PolicyYamlDiffEditor
                original={editorOriginal}
                modified={editorModified}
                onChange={setEditorModified}
                onAiEdit={handleAiEdit}
                versions={versions}
                selectedVersion={selectedVersion}
                onVersionSelect={(v) => void handleVersionSelect(v)}
                versionsLoading={versionsLoading}
                disabled={editorLoading || editorSaving}
              />
            </TabsContent>
          </Tabs>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setEditorOpen(false)}
              disabled={editorSaving}
            >
              Cancel
            </Button>
            <Button
              type="button"
              onClick={() => void saveEditor()}
              disabled={editorLoading || editorSaving || editorModified.trim() === ''}
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
