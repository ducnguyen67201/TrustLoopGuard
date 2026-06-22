'use client';

import { IconPencil, IconRobot } from '@tabler/icons-react';
import { useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';
import type { FormEvent } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogTrigger } from '@/components/ui/dialog';
import { InfoHint } from '@/components/ui/info-hint';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { getAgent, updateAgent, type AgentProfile } from '@/lib/agents';
import { isAllowedAgentTargetUrl } from '@/lib/redteam-core';
import {
  DialogFooterBar,
  DialogShellHeader,
  FieldHint,
  FormRow,
} from '@/components/workspace/dialog-scaffold';

interface AgentEditDialogProps {
  agentId: string;
  agentName: string;
}

type WorkflowParse =
  | { ok: true; value: Record<string, unknown> }
  | { ok: false; error: string };

const EMPTY_FORM = {
  displayName: '',
  systemPrompt: '',
  workflowJson: '',
  targetUrl: '',
  scopeIn: '',
  scopeOut: '',
  canPromise: '',
  cannotPromise: '',
  toneTarget: 'clear-professional',
  toneForbidden: '',
  escalationTriggers: '',
};

function parseWorkflow(raw: string): WorkflowParse | null {
  const trimmed = raw.trim();
  if (trimmed === '') return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(trimmed);
  } catch {
    return { ok: false, error: 'Invalid JSON' };
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return { ok: false, error: 'Workflow JSON must be an object' };
  }
  return { ok: true, value: parsed as Record<string, unknown> };
}

function listToText(values: string[]): string {
  return values.join('\n');
}

function textToList(value: string): string[] {
  return Array.from(
    new Set(
      value
        .split(/\r?\n|,/)
        .map((item) => item.trim())
        .filter(Boolean),
    ),
  );
}

function formFromAgent(agent: AgentProfile): typeof EMPTY_FORM {
  return {
    displayName: agent.displayName,
    systemPrompt: agent.systemPrompt ?? '',
    workflowJson:
      agent.workflowDefinition === undefined
        ? ''
        : JSON.stringify(agent.workflowDefinition.definition, null, 2),
    targetUrl: agent.targetUrl ?? '',
    scopeIn: listToText(agent.scope.inScope),
    scopeOut: listToText(agent.scope.outOfScope),
    canPromise: listToText(agent.authority.canPromise),
    cannotPromise: listToText(agent.authority.cannotPromise),
    toneTarget: agent.tone.target,
    toneForbidden: listToText(agent.tone.forbidden),
    escalationTriggers: listToText(agent.escalationTriggers),
  };
}

export function AgentEditDialog({ agentId, agentName }: AgentEditDialogProps) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState(EMPTY_FORM);
  const [workflowSource, setWorkflowSource] = useState('n8n');

  const workflowParse = parseWorkflow(form.workflowJson);
  const prompt = form.systemPrompt.trim();
  const target = form.targetUrl.trim();
  const targetOk = target === '' || isAllowedAgentTargetUrl(target);
  const promptOk = prompt === '' || prompt.length >= 20;
  const hasPrompt = prompt.length >= 20;
  const hasWorkflow = workflowParse !== null && workflowParse.ok;
  const canSave =
    !loading &&
    !saving &&
    form.displayName.trim().length > 0 &&
    targetOk &&
    promptOk &&
    form.toneTarget.trim().length > 0 &&
    (hasPrompt || hasWorkflow);

  useEffect(() => {
    if (!open) return;
    const controller = new AbortController();
    setLoading(true);
    void getAgent(agentId, controller.signal)
      .then((agent) => {
        setForm(formFromAgent(agent));
        setWorkflowSource(agent.workflowDefinition?.source ?? 'n8n');
      })
      .catch((err) => {
        if (!controller.signal.aborted) toast.error(describeError(err));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [agentId, open]);

  function patch<K extends keyof typeof EMPTY_FORM>(key: K, value: (typeof EMPTY_FORM)[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function close() {
    if (saving) return;
    setOpen(false);
    setForm(EMPTY_FORM);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSave) return;
    setSaving(true);
    try {
      const workflowDefinition =
        workflowParse !== null && workflowParse.ok
          ? { source: workflowSource.trim() || 'n8n', definition: workflowParse.value }
          : undefined;
      await updateAgent(agentId, {
        displayName: form.displayName.trim(),
        ...(hasPrompt ? { systemPrompt: prompt } : {}),
        ...(workflowDefinition !== undefined ? { workflowDefinition } : {}),
        ...(target !== '' ? { targetUrl: target } : {}),
        scope: {
          inScope: textToList(form.scopeIn),
          outOfScope: textToList(form.scopeOut),
        },
        authority: {
          canPromise: textToList(form.canPromise),
          cannotPromise: textToList(form.cannotPromise),
        },
        tone: {
          target: form.toneTarget.trim(),
          forbidden: textToList(form.toneForbidden),
        },
        escalationTriggers: textToList(form.escalationTriggers),
      });
      toast.success(`Saved "${form.displayName.trim()}"`);
      setOpen(false);
      router.refresh();
    } catch (err) {
      toast.error(describeError(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(next) => (next ? setOpen(true) : close())}>
      <DialogTrigger asChild>
        <Button variant="ghost" size="sm" className="h-8 gap-1.5">
          <IconPencil className="size-4" />
          Edit
        </Button>
      </DialogTrigger>
      <DialogContent className="flex max-h-[85vh] w-[calc(100vw-2rem)] max-w-none flex-col gap-5 overflow-y-auto sm:w-[min(68vw,calc(100vw-2rem))]">
        <DialogShellHeader
          icon={<IconRobot />}
          eyebrow="Agent config"
          title={`Edit ${agentName}`}
          description="Update the agent profile used by guardrail generation, red-team planning, and the Attacks page target picker."
        />

        <form onSubmit={handleSubmit} className="grid gap-5">
          <fieldset disabled={loading || saving} className="grid gap-4">
            <div className="grid gap-4 md:grid-cols-2">
              <FormRow>
                <Label htmlFor={`agent-${agentId}-name`}>Display name</Label>
                <Input
                  id={`agent-${agentId}-name`}
                  value={form.displayName}
                  onChange={(event) => patch('displayName', event.target.value)}
                  placeholder="NorthPay Disputes"
                  required
                />
              </FormRow>
              <FormRow>
                <Label htmlFor={`agent-${agentId}-target`}>Connection URL</Label>
                <Input
                  id={`agent-${agentId}-target`}
                  value={form.targetUrl}
                  onChange={(event) => patch('targetUrl', event.target.value)}
                  placeholder="http://127.0.0.1:9202"
                  className="font-mono"
                  aria-invalid={!targetOk}
                />
                <FieldHint invalid={!targetOk}>
                  {targetOk
                    ? 'Loopback root URL for this local demo agent.'
                    : 'Must be loopback only: 127.0.0.1, localhost, or ::1.'}
                </FieldHint>
              </FormRow>
            </div>

            <FormRow>
              <Label htmlFor={`agent-${agentId}-prompt`} className="flex items-center gap-1.5">
                System prompt
                <InfoHint term="agent" />
              </Label>
              <Textarea
                id={`agent-${agentId}-prompt`}
                value={form.systemPrompt}
                onChange={(event) => patch('systemPrompt', event.target.value)}
                rows={7}
                className="max-h-[45vh] overflow-y-auto font-mono leading-relaxed"
              />
              <FieldHint invalid={!promptOk}>
                Leave blank only if this is workflow-only. Chat prompts must be at least 20
                characters.
              </FieldHint>
            </FormRow>

            <details className="rounded-md border bg-muted/30">
              <summary className="cursor-pointer list-none px-3 py-2 text-sm font-medium">
                Workflow definition
              </summary>
              <div className="grid gap-3 border-t p-3">
                <FormRow>
                  <Label htmlFor={`agent-${agentId}-workflow-source`}>Workflow source</Label>
                  <Input
                    id={`agent-${agentId}-workflow-source`}
                    value={workflowSource}
                    onChange={(event) => setWorkflowSource(event.target.value)}
                    placeholder="n8n"
                  />
                </FormRow>
                <FormRow>
                  <Label htmlFor={`agent-${agentId}-workflow`}>Workflow JSON</Label>
                  <Textarea
                    id={`agent-${agentId}-workflow`}
                    value={form.workflowJson}
                    onChange={(event) => patch('workflowJson', event.target.value)}
                    rows={8}
                    spellCheck={false}
                    className="max-h-[45vh] overflow-y-auto font-mono text-xs leading-relaxed"
                  />
                  <FieldHint invalid={workflowParse !== null && !workflowParse.ok}>
                    {workflowParse !== null && !workflowParse.ok
                      ? workflowParse.error
                      : 'Optional. Paste workflow JSON when this agent is attacked through a document or workflow surface.'}
                  </FieldHint>
                </FormRow>
              </div>
            </details>

            <div className="grid gap-4 md:grid-cols-2">
              <FormRow>
                <Label htmlFor={`agent-${agentId}-scope-in`}>In scope</Label>
                <Textarea
                  id={`agent-${agentId}-scope-in`}
                  value={form.scopeIn}
                  onChange={(event) => patch('scopeIn', event.target.value)}
                  rows={4}
                />
                <FieldHint>One item per line, or comma-separated.</FieldHint>
              </FormRow>
              <FormRow>
                <Label htmlFor={`agent-${agentId}-scope-out`}>Out of scope</Label>
                <Textarea
                  id={`agent-${agentId}-scope-out`}
                  value={form.scopeOut}
                  onChange={(event) => patch('scopeOut', event.target.value)}
                  rows={4}
                />
              </FormRow>
              <FormRow>
                <Label htmlFor={`agent-${agentId}-can`}>Can promise</Label>
                <Textarea
                  id={`agent-${agentId}-can`}
                  value={form.canPromise}
                  onChange={(event) => patch('canPromise', event.target.value)}
                  rows={4}
                />
              </FormRow>
              <FormRow>
                <Label htmlFor={`agent-${agentId}-cannot`}>Cannot promise</Label>
                <Textarea
                  id={`agent-${agentId}-cannot`}
                  value={form.cannotPromise}
                  onChange={(event) => patch('cannotPromise', event.target.value)}
                  rows={4}
                />
              </FormRow>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <FormRow>
                <Label htmlFor={`agent-${agentId}-tone`}>Tone target</Label>
                <Input
                  id={`agent-${agentId}-tone`}
                  value={form.toneTarget}
                  onChange={(event) => patch('toneTarget', event.target.value)}
                />
              </FormRow>
              <FormRow>
                <Label htmlFor={`agent-${agentId}-forbidden`}>Forbidden tone</Label>
                <Input
                  id={`agent-${agentId}-forbidden`}
                  value={form.toneForbidden}
                  onChange={(event) => patch('toneForbidden', event.target.value)}
                  placeholder="overconfident, dismissive"
                />
              </FormRow>
            </div>

            <FormRow>
              <Label htmlFor={`agent-${agentId}-escalations`}>Escalation triggers</Label>
              <Textarea
                id={`agent-${agentId}-escalations`}
                value={form.escalationTriggers}
                onChange={(event) => patch('escalationTriggers', event.target.value)}
                rows={4}
              />
            </FormRow>
          </fieldset>

          <DialogFooterBar
            onCancel={close}
            cancelDisabled={saving}
            submitting={saving}
            submitDisabled={!canSave}
            submitLabel={loading ? 'Loading…' : 'Save changes'}
            submittingLabel="Saving…"
          />
        </form>
      </DialogContent>
    </Dialog>
  );
}

function describeError(err: unknown): string {
  if (err instanceof Error) return err.message;
  return 'Could not save agent';
}
