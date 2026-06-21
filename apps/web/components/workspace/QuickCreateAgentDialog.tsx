'use client';

import { IconRobot } from '@tabler/icons-react';
import { useRouter } from 'next/navigation';
import { useState, type ReactNode } from 'react';
import { toast } from 'sonner';

import { Dialog, DialogContent, DialogTrigger } from '@/components/ui/dialog';
import { InfoHint } from '@/components/ui/info-hint';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import { createAgent } from '@/lib/agents';
import { isAllowedAgentTargetUrl } from '@/lib/redteam-core';
import {
  DialogFooterBar,
  DialogShellHeader,
  FieldHint,
  FormRow,
} from '@/components/workspace/dialog-scaffold';

const DEFAULT_PROMPT =
  'You are a customer support agent. Answer billing and product questions, but never promise refunds, legal outcomes, or medical advice. Escalate sensitive cases to a teammate.';

const DEFAULT_TARGET = 'http://127.0.0.1:9102';

const ADAPTER_SNIPPET = `import { createArenaAdapter } from './arena/adapter';

await createArenaAdapter({
  host: '127.0.0.1',
  port: 9102,
  profile, // { displayName, systemPrompt, safeUserQuestion, protectedInformationName }
  async chat({ message }) {
    const reply = await myAgent(message);
    return { content: reply, finishReason: 'stop', verdict: null, phase: null, traceId: null };
  },
});`;

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
  const [connectionUrl, setConnectionUrl] = useState(DEFAULT_TARGET);
  const [submitting, setSubmitting] = useState(false);

  const workflowParse = workflowJson.trim() === '' ? null : parseWorkflow(workflowJson);
  const nameOk = displayName.trim().length > 0;
  const connectionOk = isAllowedAgentTargetUrl(connectionUrl.trim());
  const canSubmit =
    nameOk &&
    connectionOk &&
    (kind === 'chat'
      ? systemPrompt.trim().length >= 20
      : workflowParse !== null && workflowParse.ok);

  function reset() {
    setKind('chat');
    setDisplayName('');
    setSystemPrompt(DEFAULT_PROMPT);
    setWorkflowJson('');
    setConnectionUrl(DEFAULT_TARGET);
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
      const targetUrl = connectionUrl.trim();
      const agent =
        kind === 'chat'
          ? await createAgent({
              displayName: displayName.trim(),
              systemPrompt: systemPrompt.trim(),
              targetUrl,
            })
          : await createAgent({
              displayName: displayName.trim(),
              workflowDefinition: {
                source: 'n8n',
                // Guarded by canSubmit; narrow for the type checker.
                definition: workflowParse !== null && workflowParse.ok ? workflowParse.value : {},
              },
              targetUrl,
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
      <DialogContent className="flex max-h-[85vh] w-[calc(100vw-2rem)] max-w-none flex-col gap-5 overflow-y-auto sm:w-[min(60vw,calc(100vw-2rem))]">
        <DialogShellHeader
          icon={<IconRobot />}
          eyebrow="Import agent"
          title="Import an agent"
          description="An agent is one of your AI assistants or apps that sends requests through the guardrail. Add it by pasting its chat prompt or its workflow, and once it's saved you can run safe, simulated attacks against it from the Attacks page to see how well it holds up."
        />

        <form onSubmit={handleSubmit} className="grid gap-5">
          <fieldset disabled={submitting} className="grid gap-4">
            <FormRow>
              <Label htmlFor="quick-agent-name" className="flex items-center gap-1.5">
                Display name
                <InfoHint term="agent" />
              </Label>
              <Input
                id="quick-agent-name"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
                placeholder="Support bot"
                autoFocus
                required
              />
              <FieldHint>What you&apos;ll call this agent in the dashboard, like &ldquo;Support bot&rdquo;.</FieldHint>
            </FormRow>

            <FormRow>
              <Label htmlFor="quick-agent-kind">Source</Label>
              <Tabs
                id="quick-agent-kind"
                value={kind}
                onValueChange={(value) => setKind(value as ImportKind)}
              >
                <TabsList className="w-full">
                  <TabsTrigger value="chat">Chat prompt</TabsTrigger>
                  <TabsTrigger value="workflow">Workflow JSON</TabsTrigger>
                </TabsList>
              </Tabs>
            </FormRow>

            {kind === 'chat' ? (
              <FormRow>
                <Label htmlFor="quick-agent-prompt">System prompt</Label>
                <Textarea
                  id="quick-agent-prompt"
                  value={systemPrompt}
                  onChange={(event) => setSystemPrompt(event.target.value)}
                  rows={6}
                  className="max-h-[45vh] overflow-y-auto font-mono leading-relaxed"
                />
                <FieldHint>
                  Minimum 20 characters. Describe the agent&apos;s purpose and the topics it must
                  avoid or escalate.
                </FieldHint>
              </FormRow>
            ) : (
              <FormRow>
                <Label htmlFor="quick-agent-workflow">Workflow definition (n8n JSON)</Label>
                <Textarea
                  id="quick-agent-workflow"
                  value={workflowJson}
                  onChange={(event) => setWorkflowJson(event.target.value)}
                  rows={8}
                  spellCheck={false}
                  placeholder={WORKFLOW_PLACEHOLDER}
                  // field-sizing-content auto-grows the textarea; cap it so a
                  // long workflow scrolls inside instead of pushing the footer off-screen.
                  className="max-h-[45vh] overflow-y-auto font-mono text-xs leading-relaxed"
                />
                <FieldHint invalid={workflowParse !== null && !workflowParse.ok}>
                  {workflowParse !== null && !workflowParse.ok
                    ? workflowParse.error
                    : 'Paste an n8n workflow export. The planner walks its nodes → source→sink paths to tailor attacks.'}
                </FieldHint>
              </FormRow>
            )}

            <FormRow>
              <Label htmlFor="quick-agent-url">Connection — agent URL</Label>
              <Input
                id="quick-agent-url"
                value={connectionUrl}
                onChange={(event) => setConnectionUrl(event.target.value)}
                placeholder={DEFAULT_TARGET}
                className="font-mono"
                aria-invalid={!connectionOk}
              />
              <details className="min-w-0 overflow-hidden rounded-md border bg-muted/40 text-xs">
                <summary className="cursor-pointer list-none px-3 py-2 font-medium">
                  How to expose your agent
                </summary>
                <pre className="max-w-full overflow-x-auto border-t px-3 py-2 font-mono leading-5">
                  {ADAPTER_SNIPPET}
                </pre>
              </details>
              <FieldHint invalid={!connectionOk}>
                {connectionOk
                  ? 'Where the Attacks page will reach this agent. Loopback only (127.0.0.1 / localhost).'
                  : 'Must be a loopback URL (127.0.0.1, localhost, or ::1).'}
              </FieldHint>
            </FormRow>
          </fieldset>

          <DialogFooterBar
            onCancel={() => handleOpenChange(false)}
            cancelDisabled={submitting}
            submitting={submitting}
            submitDisabled={!canSubmit}
            submitLabel="Create agent"
            submittingLabel="Creating…"
          />
        </form>
      </DialogContent>
    </Dialog>
  );
}

function describeError(err: unknown): string {
  if (err instanceof Error) return err.message;
  return 'Could not create agent';
}
