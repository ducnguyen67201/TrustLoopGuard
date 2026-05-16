'use client';

import { useRouter } from 'next/navigation';
import { useState, type ComponentProps, type ReactNode } from 'react';

import { Button } from '@/components/ui/button';
import { PolicyEditorDialog } from '@/components/policies/PolicyEditorDialog';
import type { PolicyDraft } from '@/lib/policy-draft';
import type { AgentRow } from '@/lib/server/dashboard-data';

export function PolicyCreateDialog({
  agents,
  workspaceSlug,
  variant,
  children,
}: {
  agents: AgentRow[];
  workspaceSlug: string;
  variant?: ComponentProps<typeof Button>['variant'];
  children: ReactNode;
}) {
  const router = useRouter();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [agentId, setAgentId] = useState('global');
  const [enabled, setEnabled] = useState(true);

  function openDialog() {
    setAgentId('global');
    setEnabled(true);
    setDialogOpen(true);
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
      <PolicyEditorDialog
        open={dialogOpen}
        mode={{ kind: 'create' }}
        onOpenChange={setDialogOpen}
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
    </>
  );
}
