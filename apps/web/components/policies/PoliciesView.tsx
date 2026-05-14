'use client';

import { IconDotsVertical, IconPlus, IconShieldCheck } from '@tabler/icons-react';
import { Loader2 } from 'lucide-react';
import { useEffect, useState } from 'react';
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
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { deletePolicy, listPolicies, setPolicyEnabled } from '@/lib/policies';
import { GenerateForAgentDialog } from './GenerateForAgentDialog';
import { PolicyEditorDialog } from './PolicyEditorDialog';

const SEVERITY_VARIANT: Record<Severity, 'secondary' | 'default' | 'outline' | 'destructive'> = {
  low: 'secondary',
  medium: 'default',
  high: 'outline',
  critical: 'destructive',
};

type EditorMode = { kind: 'create' } | { kind: 'edit'; policyId: string };

export function PoliciesView() {
  const [policies, setPolicies] = useState<PolicySummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editor, setEditor] = useState<{ open: boolean; mode: EditorMode }>({
    open: false,
    mode: { kind: 'create' },
  });
  const [deleting, setDeleting] = useState<PolicySummary | null>(null);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [generateOpen, setGenerateOpen] = useState(false);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const result = await listPolicies();
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

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
            {policies.length} policies
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={() => setGenerateOpen(true)}>
            <IconShieldCheck />
            Auto-generate from agent
          </Button>
          <Button onClick={() => setEditor({ open: true, mode: { kind: 'create' } })}>
            <IconPlus />
            New policy
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
            <Button variant="outline" onClick={refresh}>
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
              Create your first guardrail policy. Describe it in plain English and we'll draft it for you.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => setEditor({ open: true, mode: { kind: 'create' } })}>
              <IconPlus />
              New policy
            </Button>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="p-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="font-mono text-[10px] uppercase tracking-wider">id</TableHead>
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
                      <Badge variant={SEVERITY_VARIANT[policy.severity]} className="font-mono uppercase">
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

      <GenerateForAgentDialog
        open={generateOpen}
        onOpenChange={setGenerateOpen}
        onGenerated={() => {
          void refresh();
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
