'use client';

import { Loader2 } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useState, type ReactNode } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';
import { createAgent } from '@/lib/agents';

const DEFAULT_PROMPT =
  'You are a customer support agent. Answer billing and product questions, but never promise refunds, legal outcomes, or medical advice. Escalate sensitive cases to a teammate.';

const WORKFLOW_PLACEHOLDER = `{
  "nodes": [
    { "name": "Inbox", "type": "n8n-nodes-base.emailReadImap", "parameters": {} },
    { "name": "Send", "type": "n8n-nodes-base.httpRequest", "parameters": { "url": "..." } }
  ],
  "connections": { "Inbox": { "main": [[{ "node": "Send" }]] } }
}`;

type ImportKind = 'chat' | 'workflow';

type WorkflowParse =
  | { ok: true; value: Record<string, unknown> }
  | { ok: false; error: string };

/** Parse pasted n8n JSON. Requires an object with a `nodes` array so the
 *  attack planner has a graph to analyse. */
function parseWorkflow(raw: string): WorkflowParse {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return { ok: false, error: 'Invalid JSON' };
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return { ok: false, error: 'Workflow JSON must be an object' };
  }
  if (!Array.isArray((parsed as { nodes?: unknown }).nodes)) {
    return { ok: false, error: 'Workflow JSON must include a "nodes" array (n8n export)' };
  }
  return { ok: true, value: parsed as Record<string, unknown> };
}

interface QuickCreateAgentDialogProps {
  children: ReactNode;
}

export function QuickCreateAgentDialog({ children }: QuickCreateAgentDialogProps) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [kind, setKind] = useState<ImportKind>('chat');
  const [displayName, setDisplayName] = useState('');
  const [systemPrompt, setSystemPrompt] = useState(DEFAULT_PROMPT);
  const [workflowJson, setWorkflowJson] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const workflowParse = workflowJson.trim() === '' ? null : parseWorkflow(workflowJson);
  const nameOk = displayName.trim().length > 0;
  const canSubmit =
    nameOk &&
    (kind === 'chat'
      ? systemPrompt.trim().length >= 20
      : workflowParse !== null && workflowParse.ok);

  function reset() {
    setKind('chat');
    setDisplayName('');
    setSystemPrompt(DEFAULT_PROMPT);
    setWorkflowJson('');
    setSubmitting(false);
  }

  function handleOpenChange(next: boolean) {
    if (submitting) return;
    setOpen(next);
    if (!next) reset();
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || submitting) return;
    setSubmitting(true);
    try {
      const agent =
        kind === 'chat'
          ? await createAgent({
              displayName: displayName.trim(),
              systemPrompt: systemPrompt.trim(),
            })
          : await createAgent({
              displayName: displayName.trim(),
              workflowDefinition: {
                source: 'n8n',
                // Guarded by canSubmit; narrow for the type checker.
                definition: workflowParse !== null && workflowParse.ok ? workflowParse.value : {},
              },
            });
      toast.success(`Created agent "${agent.displayName}"`);
      setOpen(false);
      reset();
      router.refresh();
    } catch (err) {
      toast.error(describeError(err));
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Import agent</DialogTitle>
          <DialogDescription>
            Import a chat agent by its system prompt, or a workflow agent by its definition. The
            hardening loop derives tailored attacks from whichever you provide.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="grid gap-4">
          <fieldset disabled={submitting} className="grid gap-4">
            <div className="grid gap-2">
              <Label htmlFor="quick-agent-name">Display name</Label>
              <Input
                id="quick-agent-name"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
                placeholder="Support bot"
                autoFocus
                required
              />
            </div>

            <div
              role="tablist"
              aria-label="Agent kind"
              className="grid grid-cols-2 gap-1 rounded-lg bg-muted p-1"
            >
              {(['chat', 'workflow'] as const).map((option) => (
                <button
                  key={option}
                  type="button"
                  role="tab"
                  aria-selected={kind === option}
                  onClick={() => setKind(option)}
                  className={cn(
                    'rounded-md px-3 py-1.5 text-sm font-medium transition-colors',
                    kind === option
                      ? 'bg-background text-foreground shadow-sm'
                      : 'text-muted-foreground hover:text-foreground',
                  )}
                >
                  {option === 'chat' ? 'Chat prompt' : 'Workflow JSON'}
                </button>
              ))}
            </div>

            {kind === 'chat' ? (
              <div className="grid gap-2">
                <Label htmlFor="quick-agent-prompt">System prompt</Label>
                <Textarea
                  id="quick-agent-prompt"
                  value={systemPrompt}
                  onChange={(event) => setSystemPrompt(event.target.value)}
                  rows={6}
                  className="font-mono leading-relaxed"
                />
                <p className="text-xs text-muted-foreground">
                  Minimum 20 characters. Describe the agent&apos;s purpose and the topics it must
                  avoid or escalate.
                </p>
              </div>
            ) : (
              <div className="grid gap-2">
                <Label htmlFor="quick-agent-workflow">Workflow definition (n8n JSON)</Label>
                <Textarea
                  id="quick-agent-workflow"
                  value={workflowJson}
                  onChange={(event) => setWorkflowJson(event.target.value)}
                  rows={8}
                  spellCheck={false}
                  placeholder={WORKFLOW_PLACEHOLDER}
                  className="font-mono text-xs leading-relaxed"
                />
                <p
                  className={cn(
                    'text-xs',
                    workflowParse !== null && !workflowParse.ok
                      ? 'text-destructive'
                      : 'text-muted-foreground',
                  )}
                >
                  {workflowParse !== null && !workflowParse.ok
                    ? workflowParse.error
                    : 'Paste an n8n workflow export. The planner walks its nodes → source→sink paths to tailor attacks.'}
                </p>
              </div>
            )}
          </fieldset>

          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => handleOpenChange(false)}
              disabled={submitting}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={!canSubmit || submitting}>
              {submitting ? (
                <>
                  <Loader2 className="size-4 animate-spin" />
                  Creating…
                </>
              ) : (
                'Create agent'
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function describeError(err: unknown): string {
  if (err instanceof Error) return err.message;
  return 'Could not create agent';
}
