'use client';

import { useRouter } from 'next/navigation';
import { useState, type ComponentProps, type ReactNode } from 'react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { PolicyEditorDialog } from '@/components/policies/PolicyEditorDialog';
import type { PolicyDraft } from '@/lib/policy-draft';
import type { AgentRow } from '@/lib/server/dashboard-data';
import { FinancialPolicyCreateDialog } from './FinancialSpendingControlsCard';

export function PolicyCreateDialog({
  agents,
  workspaceSlug,
  contextQuery,
  variant,
  children,
}: {
  agents: AgentRow[];
  workspaceSlug: string;
  contextQuery: string;
  variant?: ComponentProps<typeof Button>['variant'];
  children: ReactNode;
}) {
  const router = useRouter();
  const [chooserOpen, setChooserOpen] = useState(false);
  const [protectionOpen, setProtectionOpen] = useState(false);
  const [financialOpen, setFinancialOpen] = useState(false);
  const [agentId, setAgentId] = useState('global');
  const [enabled, setEnabled] = useState(true);

  function openDialog() {
    setAgentId('global');
    setEnabled(true);
    setChooserOpen(true);
  }

  async function saveDraft(
    draft: PolicyDraft,
    options: { agentId: string | null; enabled: boolean },
  ): Promise<string> {
    const res = await fetch('/api/workspace-policies', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        workspace: workspaceSlug,
        draft,
        agentId: options.agentId,
        enabled: options.enabled,
      }),
    });

    if (!res.ok) {
      const body = (await res.json().catch(() => null)) as { error?: unknown } | null;
      throw new Error(typeof body?.error === 'string' ? body.error : 'Could not save policy');
    }

    const body = (await res.json()) as { policyId: string };
    return body.policyId;
  }

  return (
    <>
      <Button variant={variant} onClick={openDialog}>
        {children}
      </Button>
      <Dialog open={chooserOpen} onOpenChange={setChooserOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Create policy</DialogTitle>
            <DialogDescription>
              Pick the policy family. Every policy is saved to the same registry and appears in the
              same table.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3 md:grid-cols-2">
            <button
              type="button"
              className="grid gap-2 rounded-lg border bg-background p-4 text-left transition-colors hover:bg-muted/50"
              onClick={() => {
                setChooserOpen(false);
                setProtectionOpen(true);
              }}
            >
              <span className="text-sm font-medium">Protection policy</span>
              <span className="text-sm text-muted-foreground">
                Match content, requests, or tool traffic and allow, rewrite, block, or escalate.
              </span>
            </button>
            <button
              type="button"
              className="grid gap-2 rounded-lg border bg-background p-4 text-left transition-colors hover:bg-muted/50"
              onClick={() => {
                setChooserOpen(false);
                setFinancialOpen(true);
              }}
            >
              <span className="text-sm font-medium">Financial authorization</span>
              <span className="text-sm text-muted-foreground">
                Set caps, holds, evidence checks, and approval behavior for agent financial actions.
              </span>
            </button>
          </div>
        </DialogContent>
      </Dialog>
      <PolicyEditorDialog
        open={protectionOpen}
        mode={{ kind: 'create' }}
        onOpenChange={setProtectionOpen}
        onSaveDraft={saveDraft}
        onSaved={() => {
          router.refresh();
        }}
        agents={agents}
        selectedAgentId={agentId}
        onSelectedAgentIdChange={setAgentId}
        enabled={enabled}
        onEnabledChange={setEnabled}
        showValidate={false}
      />
      <FinancialPolicyCreateDialog
        open={financialOpen}
        onOpenChange={setFinancialOpen}
        contextQuery={contextQuery}
        onCreated={() => {
          router.refresh();
        }}
      />
    </>
  );
}
