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
import { InfoHint } from '@/components/ui/info-hint';
import { PageHeader } from '@/components/ui/page-header';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { VerdictLegend } from '@/components/ui/verdict-legend';
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

// Friendly, capitalized label + one-line meaning for each action so the table
// reads like English rather than lowercase tokens. Mirrors lib/glossary verdicts.
const ACTION_LABEL: Record<string, string> = {
  allow: 'Allow',
  rewrite: 'Rewrite',
  block: 'Block',
  escalate: 'Escalate',
};

const ACTION_HELP: Record<string, string> = {
  allow: 'Lets the request through unchanged.',
  rewrite: 'Cleans up the request, then lets it through.',
  block: 'Stops the request when this rule matches.',
  escalate: 'Holds the request for a person to review.',
};

function ActionBadge({ action }: { action: string }) {
  const key = action.toLowerCase();
  if (VERDICT_ACTIONS.has(key)) {
    return (
      <Badge variant={key as VerdictVariant} title={ACTION_HELP[key]}>
        {ACTION_LABEL[key] ?? action}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" title={ACTION_HELP[key]}>
      {ACTION_LABEL[key] ?? action}
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
  const deleteCount = deleteTarget?.ids.length ?? 0;
  const selectedPolicies = policies.filter((policy) => selectedIdSet.has(policy.id));
  const enabledCount = useMemo(
    () => policies.filter((policy) => policy.enabled).length,
    [policies],
  );

  const policyColumns: DataTableColumn<PolicyRow>[] = [
    {
      id: 'id',
      header: 'Protection rule',
      cell: (row) => {
        const hasName = row.description.trim() !== '';
        return (
          <div className="grid min-w-0 gap-0.5">
            <span className="truncate text-sm font-medium text-foreground">
              {hasName ? row.description : row.id}
            </span>
            {hasName ? (
              <span className="truncate font-mono text-xs text-muted-foreground">{row.id}</span>
            ) : null}
          </div>
        );
      },
    },
    {
      id: 'agent',
      header: (
        <span className="inline-flex items-center gap-1">
          Applies to
          <InfoHint>Which AI assistant this rule checks. “Global” means all of them.</InfoHint>
        </span>
      ),
      cell: (row) => <span className="text-sm">{row.agent}</span>,
    },
    {
      id: 'severity',
      header: (
        <span className="inline-flex items-center gap-1">
          Severity
          <InfoHint term="severity" />
        </span>
      ),
      cell: (row) => <PolicySeverityBadge severity={row.severity} />,
    },
    {
      id: 'action',
      header: (
        <span className="inline-flex items-center gap-1">
          On a match
          <InfoHint term="verdict" />
        </span>
      ),
      cell: (row) => <ActionBadge action={row.action} />,
    },
    {
      id: 'enabled',
      header: (
        <span className="inline-flex items-center gap-1">
          Status
          <InfoHint>On = this rule is actively checking traffic right now. Off = saved but paused.</InfoHint>
        </span>
      ),
      align: 'right',
      cell: (row) => (
        <div className="flex items-center justify-end gap-2">
          <span
            className={
              row.enabled
                ? 'text-xs font-medium text-foreground'
                : 'text-xs text-muted-foreground'
            }
          >
            {row.enabled ? 'On' : 'Off'}
          </span>
          <TooltipProvider delayDuration={150}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Switch
                  checked={row.enabled}
                  disabled={busyIdSet.has(row.id)}
                  onCheckedChange={(enabled) => void updateOneEnabled(row.id, enabled)}
                  aria-label={
                    row.enabled
                      ? `Turn off the rule “${row.description || row.id}”`
                      : `Turn on the rule “${row.description || row.id}”`
                  }
                />
              </TooltipTrigger>
              <TooltipContent side="left">
                {row.enabled ? 'On — checking traffic now. Click to pause.' : 'Off — paused. Click to start checking.'}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
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
            <DropdownMenuItem onSelect={() => void openEditor(row.id)}>Edit rule</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              variant="destructive"
              onSelect={() =>
                setDeleteTarget({ ids: [row.id], label: row.description || row.id })
              }
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
      toast.success(enabled ? 'Rule turned on — now checking traffic' : 'Rule turned off — paused');
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
      toast.success(enabled ? 'Rules turned on' : 'Rules turned off');
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
      toast.success('Rule saved');
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
      toast.success(ids.length === 1 ? 'Rule deleted' : 'Rules deleted');
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
        title="Protection rules"
        help={<InfoHint term="policy" />}
        description={`Rules that tell the guardrail what to allow, clean up, block, or send for review. Turning a rule on starts checking ${data.activeEnvironment.name} traffic right away.`}
        actions={
          <PolicyCreateDialog agents={data.agents} workspaceSlug={data.activeWorkspace.slug}>
            <IconPlus />
            New rule
          </PolicyCreateDialog>
        }
      />

      <Card>
        <CardHeader className="flex flex-row items-center justify-between gap-3 space-y-0">
          <CardTitle>Your rules</CardTitle>
          {hasPolicies ? (
            <span className="text-xs tabular-nums text-muted-foreground">
              {enabledCount} on / {policies.length} total
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
                        label:
                          selectedPolicies.length === 1
                            ? selectedPolicies[0]?.description || selectedPolicies[0]?.id || '1 rule'
                            : `${selectedPolicies.length} rules`,
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
                caption="Your protection rules for this environment"
                empty="No protection rules yet."
              />
              <div className="mt-5 rounded-lg border bg-muted/30 p-4">
                <p className="mb-3 text-xs font-medium text-muted-foreground">
                  What the “On a match” column means
                </p>
                <VerdictLegend verdicts={['rewrite', 'escalate', 'block']} />
              </div>
            </>
          ) : (
            <EmptyState
              icon={<ShieldCheck />}
              title="Create your first protection rule"
              description={`A protection rule watches every request and decides what to do — let it through, clean it up, block it, or send it for review. Nothing is checked until you add one and turn it on for ${data.activeEnvironment.name}.`}
              action={
                <PolicyCreateDialog agents={data.agents} workspaceSlug={data.activeWorkspace.slug}>
                  <IconPlus />
                  Create a rule
                </PolicyCreateDialog>
              }
            />
          )}
        </CardContent>
      </Card>

      {/* Guided builder by default, raw YAML behind an Advanced tab */}
      <Dialog open={editorOpen} onOpenChange={(open) => { if (!open) setEditorOpen(false); }}>
        <DialogContent className="max-h-[calc(100vh-2rem)] w-[calc(100vw-2rem)] overflow-y-auto sm:max-w-5xl">
          <DialogHeader>
            <DialogTitle>Edit protection rule</DialogTitle>
            <DialogDescription>
              {editorPolicyId ? (
                <>
                  Editing <span className="font-mono">{editorPolicyId}</span>. Fill in the guided
                  form below — only switch to Advanced if you need to hand-write the rule.
                </>
              ) : (
                'Fill in the guided form below — only switch to Advanced if you need to hand-write the rule.'
              )}
            </DialogDescription>
          </DialogHeader>

          <Tabs defaultValue="builder">
            <TabsList>
              <TabsTrigger value="builder">Guided form</TabsTrigger>
              <TabsTrigger value="yaml">Advanced (YAML)</TabsTrigger>
            </TabsList>
            <TabsContent value="builder">
              <PolicyBuilderEditor
                yaml={editorModified}
                onYamlChange={setEditorModified}
                disabled={editorLoading || editorSaving}
              />
            </TabsContent>
            <TabsContent value="yaml">
              <p className="mb-3 text-xs leading-relaxed text-muted-foreground">
                This is the raw rule definition. Most people can stay on the guided form — only edit
                here if you are comfortable with YAML.
              </p>
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
              Save changes
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
            <AlertDialogTitle>
              {deleteCount > 1 ? `Delete ${deleteCount} rules?` : 'Delete this protection rule?'}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {deleteTarget?.label} will be removed for good and{' '}
              {deleteCount > 1 ? 'will stop checking traffic' : 'this rule will stop checking traffic'}.
              This can&apos;t be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep {deleteCount > 1 ? 'them' : 'it'}</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDelete}>
              {deleteCount > 1 ? `Delete ${deleteCount} rules` : 'Delete rule'}
            </AlertDialogAction>
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
