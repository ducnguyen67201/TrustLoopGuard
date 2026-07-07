'use client';

import { IconDotsVertical, IconPlus, IconTrash } from '@tabler/icons-react';
import { Power, PowerOff, Search, ShieldCheck } from 'lucide-react';
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
import { Input } from '@/components/ui/input';
import { PageHeader } from '@/components/ui/page-header';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { VerdictLegend } from '@/components/ui/verdict-legend';
import { PolicyBuilderEditor } from '@/components/policies/PolicyBuilderEditor';
import { PolicyYamlDiffEditor } from '@/components/policies/PolicyYamlDiffEditor';
import type { VersionEntry } from '@/components/policies/VersionPicker';
import { FinancialPolicyCreateDialog } from '@/components/workspace/FinancialSpendingControlsCard';
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
import type {
  AgentRow,
  DashboardShellData,
  FamilyPolicyRow,
  PolicyRow,
} from '@/lib/server/dashboard-data';
import { currentContextQuery, formatMinorUnits, titleLabel } from './financial-utils';

type PoliciesPageData = DashboardShellData & {
  agents: AgentRow[];
  policies: PolicyRow[];
  familyPolicies: FamilyPolicyRow[];
};

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

const FAMILY_LABEL: Record<string, string> = {
  content: 'Protection',
  financial: 'Financial',
};

const SUPPORTED_POLICY_FAMILIES = ['content', 'financial'] as const;

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
  const [policies, setPolicies] = useState(() =>
    mergeRegistryPolicies(data.policies, data.familyPolicies),
  );
  const [familyFilter, setFamilyFilter] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [busyIds, setBusyIds] = useState<string[]>([]);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget>(null);

  // Editor state
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorPolicyId, setEditorPolicyId] = useState<string | null>(null);
  const [editorOriginal, setEditorOriginal] = useState('');
  const [editorModified, setEditorModified] = useState('');
  const [editorLoading, setEditorLoading] = useState(false);
  const [editorSaving, setEditorSaving] = useState(false);
  const [financialEditorPolicy, setFinancialEditorPolicy] = useState<FamilyPolicyRow | null>(null);

  // Version history state
  const [versions, setVersions] = useState<VersionEntry[]>([]);
  const [versionsLoading, setVersionsLoading] = useState(false);
  const [selectedVersion, setSelectedVersion] = useState<number | null>(null);

  const { selectedIds, selectedIdSet, setSelectedIds, clearSelection } = useRowSelection();

  useEffect(() => {
    setPolicies(mergeRegistryPolicies(data.policies, data.familyPolicies));
    clearSelection();
  }, [clearSelection, data.familyPolicies, data.policies]);

  const busyIdSet = useMemo(() => new Set(busyIds), [busyIds]);
  const deleteCount = deleteTarget?.ids.length ?? 0;
  const selectedPolicies = policies.filter((policy) => selectedIdSet.has(policy.id));
  const contextQuery = currentContextQuery(data.activeWorkspace.slug, data.activeEnvironment.id);
  const genericPolicyIdSet = useMemo(
    () => new Set(data.policies.map((policy) => policy.id)),
    [data.policies],
  );
  const financialPolicyById = useMemo(
    () => new Map(data.familyPolicies.map((policy) => [policy.id, policy])),
    [data.familyPolicies],
  );
  const familyCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const policy of policies) {
      counts.set(policy.family, (counts.get(policy.family) ?? 0) + 1);
    }
    return counts;
  }, [policies]);
  const familyOptions = useMemo(() => {
    const canonicalOrder = ['all', ...SUPPORTED_POLICY_FAMILIES];
    const familyIds = new Set(policies.map((policy) => policy.family));
    const ordered = canonicalOrder.filter((family) => family === 'all' || familyIds.has(family));
    const custom = Array.from(familyIds).filter((family) => !canonicalOrder.includes(family));
    return [...ordered, ...custom].map((family) => ({
      id: family,
      label:
        family === 'all'
          ? 'All'
          : FAMILY_LABEL[family] ?? `Advanced: ${titleLabel(family)}`,
      count: family === 'all' ? policies.length : familyCounts.get(family) ?? 0,
    }));
  }, [familyCounts, policies]);
  const filteredPolicies = useMemo(
    () => {
      const query = searchQuery.trim().toLowerCase();
      return policies.filter((policy) => {
        const familyMatches = familyFilter === 'all' || policy.family === familyFilter;
        if (!familyMatches) return false;
        if (query === '') return true;
        const financialPolicy = financialPolicyById.get(policy.id);
        const searchable = [
          policy.id,
          policy.family,
          policy.description,
          policy.agent,
          policy.action,
          policy.severity,
          financialPolicy?.when?.action_kinds?.join(' '),
          financialPolicy?.when?.operations?.join(' '),
          financialPolicy?.when?.agents?.join(' '),
          financialPolicy?.required_preconditions?.join(' '),
        ]
          .filter(Boolean)
          .join(' ')
          .toLowerCase();
        return searchable.includes(query);
      });
    },
    [familyFilter, financialPolicyById, policies, searchQuery],
  );
  const enabledCount = useMemo(
    () => policies.filter((policy) => policy.enabled).length,
    [policies],
  );

  const policyColumns: DataTableColumn<PolicyRow>[] = [
    {
      id: 'id',
      header: 'Policy',
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
            {row.family === 'financial' ? (
              <FinancialPolicyDetails policy={financialPolicyById.get(row.id)} />
            ) : null}
          </div>
        );
      },
    },
    {
      id: 'family',
      header: 'Type',
      cell: (row) => (
        <Badge variant="outline">
          {FAMILY_LABEL[row.family] ?? `Advanced: ${titleLabel(row.family)}`}
        </Badge>
      ),
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
      cell: (row) => {
        const genericManaged = genericPolicyIdSet.has(row.id);
        return (
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
                    disabled={busyIdSet.has(row.id) || !genericManaged}
                    onCheckedChange={(enabled) => void updateOneEnabled(row.id, enabled)}
                    aria-label={
                      row.enabled
                        ? `Turn off the rule “${row.description || row.id}”`
                        : `Turn on the rule “${row.description || row.id}”`
                    }
                  />
                </TooltipTrigger>
                <TooltipContent side="left">
                  {!genericManaged
                    ? 'Financial policies are edited from their financial policy form.'
                    : row.enabled
                      ? 'On — checking traffic now. Click to pause.'
                      : 'Off — paused. Click to start checking.'}
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
        );
      },
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cell: (row) => {
        const financialPolicy = financialPolicyById.get(row.id);
        const genericManaged = genericPolicyIdSet.has(row.id);
        return (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm">
                <IconDotsVertical />
                <span className="sr-only">Actions</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onSelect={() =>
                  financialPolicy ? setFinancialEditorPolicy(financialPolicy) : void openEditor(row.id)
                }
              >
                Edit policy
              </DropdownMenuItem>
              {genericManaged ? (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    variant="destructive"
                    onSelect={() =>
                      setDeleteTarget({ ids: [row.id], label: row.description || row.id })
                    }
                  >
                    Delete
                  </DropdownMenuItem>
                </>
              ) : null}
            </DropdownMenuContent>
          </DropdownMenu>
        );
      },
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
      toast.success(
        enabled ? 'Policy turned on — now checking traffic' : 'Policy turned off — paused',
      );
      router.refresh();
    } catch (err) {
      setPolicies(previous);
      toast.error(describeError(err));
    } finally {
      setBusyIds((prev) => prev.filter((id) => id !== policyId));
    }
  }

  async function updateSelectedEnabled(enabled: boolean) {
    const ids = selectedPolicies
      .filter((policy) => genericPolicyIdSet.has(policy.id))
      .map((policy) => policy.id);
    if (ids.length === 0) return;
    setBusyIds((prev) => Array.from(new Set([...prev, ...ids])));
    const previous = policies;
    setPolicies((prev) =>
      prev.map((policy) => (ids.includes(policy.id) ? { ...policy, enabled } : policy)),
    );
    try {
      await setPoliciesEnabled(ids, enabled);
      clearSelection();
      toast.success(enabled ? 'Policies turned on' : 'Policies turned off');
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
      toast.success('Policy saved');
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
        title="Policy registry"
        help={<InfoHint term="policy" />}
        description={`One registry for protection and financial authorization policies in ${data.activeEnvironment.name}.`}
        actions={
          <PolicyCreateDialog
            agents={data.agents}
            workspaceSlug={data.activeWorkspace.slug}
            contextQuery={contextQuery}
          >
            <IconPlus />
            New policy
          </PolicyCreateDialog>
        }
      />

      <Card>
        <CardHeader className="flex flex-row items-center justify-between gap-3 space-y-0">
          <CardTitle>Registry</CardTitle>
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
                            ? selectedPolicies[0]?.description || selectedPolicies[0]?.id || '1 policy'
                            : `${selectedPolicies.length} policies`,
                      }),
                  },
                ]}
                className="mb-3"
              />
              <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                <div className="flex flex-wrap gap-2" aria-label="Policy family filter">
                  {familyOptions.map((option) => (
                    <Button
                      key={option.id}
                      type="button"
                      size="sm"
                      variant={familyFilter === option.id ? 'default' : 'outline'}
                      onClick={() => setFamilyFilter(option.id)}
                    >
                      {option.label}
                      <span className="font-mono text-xs tabular-nums">{option.count}</span>
                    </Button>
                  ))}
                </div>
                <div className="relative w-full lg:max-w-sm">
                  <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    value={searchQuery}
                    onChange={(event) => setSearchQuery(event.target.value)}
                    placeholder="Search policies"
                    className="pl-9"
                    aria-label="Search policies"
                  />
                </div>
              </div>
              <DataTable
                columns={policyColumns}
                rows={filteredPolicies}
                getRowKey={(policy) => policy.id}
                selection={{
                  selectedRowKeys: selectedIds,
                  onSelectedRowKeysChange: setSelectedIds,
                  getRowCanSelect: (policy) => genericPolicyIdSet.has(policy.id),
                }}
                caption="Policy registry for this environment"
                empty={searchQuery.trim() ? 'No policies match your search.' : 'No policies yet.'}
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
              title="Create your first policy"
              description={`Policies watch requests and decide what to do: allow, clean up, deny, hold for review, or authorize a financial action. Nothing is checked until you add one and turn it on for ${data.activeEnvironment.name}.`}
              action={
                <PolicyCreateDialog
                  agents={data.agents}
                  workspaceSlug={data.activeWorkspace.slug}
                  contextQuery={contextQuery}
                >
                  <IconPlus />
                  Create a policy
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
            <DialogTitle>Edit policy</DialogTitle>
            <DialogDescription>
              {editorPolicyId ? (
                <>
                  Editing <span className="font-mono">{editorPolicyId}</span>. Fill in the guided
                  form below — only switch to Advanced if you need to hand-write the policy.
                </>
              ) : (
                'Fill in the guided form below — only switch to Advanced if you need to hand-write the policy.'
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
                This is the raw policy definition. Most people can stay on the guided form — only edit
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

      <FinancialPolicyCreateDialog
        open={financialEditorPolicy !== null}
        onOpenChange={(open) => {
          if (!open) setFinancialEditorPolicy(null);
        }}
        contextQuery={contextQuery}
        initialPolicy={financialEditorPolicy ?? undefined}
        existingPolicyIds={data.familyPolicies.map((policy) => policy.id)}
        onCreated={() => {
          router.refresh();
        }}
      />

      <AlertDialog
        open={deleteTarget !== null}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {deleteCount > 1 ? `Delete ${deleteCount} policies?` : 'Delete this policy?'}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {deleteTarget?.label} will be removed for good and{' '}
              {deleteCount > 1 ? 'will stop checking traffic' : 'this policy will stop checking traffic'}.
              This can&apos;t be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep {deleteCount > 1 ? 'them' : 'it'}</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDelete}>
              {deleteCount > 1 ? `Delete ${deleteCount} policies` : 'Delete policy'}
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

function mergeRegistryPolicies(
  policies: PolicyRow[],
  familyPolicies: FamilyPolicyRow[],
): PolicyRow[] {
  const policyIds = new Set(policies.map((policy) => policy.id));
  const financialRows = familyPolicies
    .filter((policy) => !policyIds.has(policy.id))
    .map((policy) => ({
      id: policy.id,
      family: 'financial',
      description: policy.description?.trim() || financialPolicyScope(policy),
      severity: policy.severity ?? 'high',
      action: policy.hold_above_minor != null ? 'escalate' : policy.on_breach ?? 'block',
      enabled: policy.enabled ?? true,
      agent: policy.when?.agents?.[0] ?? 'Global',
    }));
  return [...financialRows, ...policies];
}

function FinancialPolicyDetails({ policy }: { policy: FamilyPolicyRow | undefined }) {
  if (!policy) {
    return (
      <span className="text-xs text-muted-foreground">
        Financial authorization policy
      </span>
    );
  }
  const currency = policy.when?.currencies?.[0] ?? 'USD';
  return (
    <div className="mt-1 flex flex-wrap gap-1.5">
      <Badge variant="outline" className="text-xs">
        {financialPolicyScope(policy)}
      </Badge>
      <FinancialCap label="per action" value={policy.per_transaction_minor} currency={currency} />
      <FinancialCap label="daily" value={policy.daily_minor} currency={currency} />
      <FinancialCap label="monthly" value={policy.monthly_minor} currency={currency} />
      <FinancialCap label="hold" value={policy.hold_above_minor} currency={currency} />
      {policy.required_preconditions?.length ? (
        <Badge variant="outline" className="text-xs">
          {policy.required_preconditions.length} evidence checks
        </Badge>
      ) : null}
    </div>
  );
}

function FinancialCap({
  label,
  value,
  currency,
}: {
  label: string;
  value: number | null | undefined;
  currency: string;
}) {
  if (value == null) return null;
  return (
    <Badge variant="outline" className="font-mono text-xs tabular-nums">
      {label} {formatMinorUnits(value, currency)}
    </Badge>
  );
}

function financialPolicyScope(policy: FamilyPolicyRow): string {
  const kind = policy.when?.action_kinds?.[0] ?? 'financial action';
  const agent = policy.when?.agents?.[0] ?? 'all agents';
  return `${titleLabel(kind)} for ${agent}`;
}
