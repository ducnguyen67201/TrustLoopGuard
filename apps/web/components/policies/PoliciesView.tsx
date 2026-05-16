'use client';

import { IconDotsVertical, IconRobot, IconShieldCheck } from '@tabler/icons-react';
import { Loader2, Sparkles } from 'lucide-react';
import { useEffect, useState } from 'react';
import { usePathname, useRouter, useSearchParams } from 'next/navigation';
import { toast } from 'sonner';
import type { PolicySummary, Severity } from '@trustloopguard/sdk';
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
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { listAgents, type AgentSummary } from '@/lib/agents';
import { deletePolicy, listPolicies, listPoliciesForAgent, setPolicyEnabled } from '@/lib/policies';
import { AgentGuardrailDialog } from './AgentGuardrailDialog';
import { PolicyEditorDialog } from './PolicyEditorDialog';

const SEVERITY_VARIANT: Record<Severity, 'secondary' | 'default' | 'outline' | 'destructive'> = {
  low: 'secondary',
  medium: 'default',
  high: 'outline',
  critical: 'destructive',
};

type EditorMode = { kind: 'create' } | { kind: 'edit'; policyId: string };
const ALL_AGENTS_VALUE = '__all__';

export function PoliciesView({ workspaceName }: { workspaceName: string }) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const selectedAgentId = searchParams.get('agentid')?.trim() || null;

  const [policies, setPolicies] = useState<PolicySummary[]>([]);
  const [agents, setAgents] = useState<AgentSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [agentsLoading, setAgentsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editor, setEditor] = useState<{ open: boolean; mode: EditorMode }>({
    open: false,
    mode: { kind: 'create' },
  });
  const [agentDialogOpen, setAgentDialogOpen] = useState(false);
  const [deleting, setDeleting] = useState<PolicySummary | null>(null);
  const [togglingId, setTogglingId] = useState<string | null>(null);

  useEffect(() => {
    void refresh(selectedAgentId);
  }, [selectedAgentId]);

  useEffect(() => {
    let cancelled = false;
    setAgentsLoading(true);
    (async () => {
      try {
        const result = await listAgents();
        if (!cancelled) setAgents(result.agents);
      } catch (err) {
        if (!cancelled) toast.error(`Could not load agents: ${describeError(err)}`);
      } finally {
        if (!cancelled) setAgentsLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function refresh(agentId: string | null = selectedAgentId) {
    setLoading(true);
    setError(null);
    try {
      const result = agentId !== null ? await listPoliciesForAgent(agentId) : await listPolicies();
      setPolicies(result.policies);
    } catch (err) {
      setError(describeError(err));
    } finally {
      setLoading(false);
    }
  }

  async function toggleEnabled(policy: PolicySummary) {
    setTogglingId(policy.id);
    setPolicies((prev) =>
      prev.map((p) => (p.id === policy.id ? { ...p, enabled: !p.enabled } : p)),
    );
    try {
      await setPolicyEnabled(policy.id, !policy.enabled);
      toast.success(policy.enabled ? 'Policy disabled' : 'Policy enabled');
    } catch (err) {
      setPolicies((prev) =>
        prev.map((p) => (p.id === policy.id ? { ...p, enabled: policy.enabled } : p)),
      );
      toast.error(describeError(err));
    } finally {
      setTogglingId(null);
    }
  }

  async function confirmDelete() {
    if (!deleting) return;
    const target = deleting;
    setDeleting(null);
    try {
      await deletePolicy(target.id);
      toast.success(`Deleted ${target.id}`);
      await refresh();
    } catch (err) {
      toast.error(describeError(err));
    }
  }

  function selectAgent(nextValue: string) {
    const params = new URLSearchParams(searchParams.toString());
    if (nextValue === ALL_AGENTS_VALUE) {
      params.delete('agentid');
    } else {
      params.set('agentid', nextValue);
    }
    const query = params.toString();
    router.replace(query === '' ? pathname : `${pathname}?${query}`);
  }

  const selectedAgent = agents.find((agent) => agent.agentId === selectedAgentId) ?? null;
  const policyCountLabel =
    selectedAgentId === null
      ? `${policies.length} policies`
      : `${policies.length} policies for ${selectedAgent?.displayName ?? selectedAgentId}`;

  return (
    <div className="space-y-4 px-4 lg:px-6">
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
            {policyCountLabel}
          </p>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <Badge variant="outline" className="rounded-sm">
              workspace: {workspaceName}
            </Badge>
            <Badge variant="outline" className="rounded-sm">
              draft then enable
            </Badge>
          </div>
          {selectedAgentId !== null ? (
            <p className="mt-1 font-mono text-xs text-muted-foreground">
              /policies?agentid={selectedAgentId}
            </p>
          ) : null}
        </div>
        <div className="flex flex-col gap-2 sm:flex-row">
          <Select
            value={selectedAgentId ?? ALL_AGENTS_VALUE}
            onValueChange={selectAgent}
            disabled={agentsLoading}
          >
            <SelectTrigger className="w-full sm:w-64">
              <SelectValue placeholder={agentsLoading ? 'Loading agents...' : 'All agents'} />
            </SelectTrigger>
            <SelectContent align="end">
              <SelectItem value={ALL_AGENTS_VALUE}>All agents</SelectItem>
              {selectedAgentId !== null && selectedAgent === null ? (
                <SelectItem value={selectedAgentId}>{selectedAgentId}</SelectItem>
              ) : null}
              {agents.map((agent) => (
                <SelectItem key={agent.agentId} value={agent.agentId}>
                  {agent.displayName}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="outline" onClick={() => setAgentDialogOpen(true)}>
            <IconRobot />
            Generate from agent
          </Button>
          <Button onClick={() => setEditor({ open: true, mode: { kind: 'create' } })}>
            <Sparkles />
            Generate policy
          </Button>
        </div>
      </div>

      {error !== null ? (
        <Card>
          <CardHeader>
            <CardTitle>Could not load policies</CardTitle>
            <CardDescription className="font-mono text-xs">{error}</CardDescription>
          </CardHeader>
          <CardContent>
            <Button variant="outline" onClick={() => void refresh()}>
              Retry
            </Button>
          </CardContent>
        </Card>
      ) : loading ? (
        <Card>
          <CardContent className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
            <Loader2 className="animate-spin" />
            Loading policies…
          </CardContent>
        </Card>
      ) : policies.length === 0 ? (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <IconShieldCheck className="size-5" />
              No policies yet
            </CardTitle>
            <CardDescription>
              Create your first guardrail policy. Describe it in plain English and we'll draft it
              for you.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => setEditor({ open: true, mode: { kind: 'create' } })}>
              <Sparkles />
              Generate policy
            </Button>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="p-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="font-mono text-[10px] uppercase tracking-wider">
                    id
                  </TableHead>
                  <TableHead className="font-mono text-[10px] uppercase tracking-wider">
                    description
                  </TableHead>
                  <TableHead className="font-mono text-[10px] uppercase tracking-wider">
                    severity
                  </TableHead>
                  <TableHead className="font-mono text-[10px] uppercase tracking-wider">
                    enabled
                  </TableHead>
                  <TableHead />
                </TableRow>
              </TableHeader>
              <TableBody>
                {policies.map((policy) => (
                  <TableRow key={policy.id}>
                    <TableCell className="font-mono text-xs">{policy.id}</TableCell>
                    <TableCell className="max-w-md text-sm text-muted-foreground">
                      {policy.description ?? '—'}
                    </TableCell>
                    <TableCell>
                      <Badge
                        variant={SEVERITY_VARIANT[policy.severity]}
                        className="font-mono uppercase"
                      >
                        {policy.severity}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Switch
                        checked={policy.enabled}
                        disabled={togglingId === policy.id}
                        onCheckedChange={() => void toggleEnabled(policy)}
                        aria-label={`Toggle ${policy.id}`}
                      />
                    </TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <IconDotsVertical />
                            <span className="sr-only">Actions</span>
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onSelect={() =>
                              setEditor({ open: true, mode: { kind: 'edit', policyId: policy.id } })
                            }
                          >
                            Edit
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            variant="destructive"
                            onSelect={() => setDeleting(policy)}
                          >
                            Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}

      <PolicyEditorDialog
        open={editor.open}
        mode={editor.mode}
        onOpenChange={(open) => setEditor((prev) => ({ ...prev, open }))}
        onSaved={() => {
          void refresh();
        }}
      />

      <AgentGuardrailDialog
        open={agentDialogOpen}
        onOpenChange={setAgentDialogOpen}
        onGenerated={(agent) => {
          setAgents((prev) =>
            prev.some((existing) => existing.agentId === agent.agentId)
              ? prev
              : [...prev, { ...agent, hasSystemPrompt: true }],
          );
          selectAgent(agent.agentId);
        }}
      />

      <AlertDialog open={deleting !== null} onOpenChange={(open) => !open && setDeleting(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete policy?</AlertDialogTitle>
            <AlertDialogDescription>
              This removes <span className="font-mono">{deleting?.id}</span> from tl-server. This
              action can't be undone.
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
