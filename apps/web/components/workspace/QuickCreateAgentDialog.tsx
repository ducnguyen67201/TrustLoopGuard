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
import { createAgent } from '@/lib/agents';

const DEFAULT_PROMPT =
  'You are a customer support agent. Answer billing and product questions, but never promise refunds, legal outcomes, or medical advice. Escalate sensitive cases to a teammate.';

interface QuickCreateAgentDialogProps {
  children: ReactNode;
}

export function QuickCreateAgentDialog({ children }: QuickCreateAgentDialogProps) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [displayName, setDisplayName] = useState('');
  const [systemPrompt, setSystemPrompt] = useState(DEFAULT_PROMPT);
  const [submitting, setSubmitting] = useState(false);

  const canSubmit = displayName.trim().length > 0 && systemPrompt.trim().length >= 20;

  function reset() {
    setDisplayName('');
    setSystemPrompt(DEFAULT_PROMPT);
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
      const agent = await createAgent({
        displayName: displayName.trim(),
        systemPrompt: systemPrompt.trim(),
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
          <DialogTitle>Quick create agent</DialogTitle>
          <DialogDescription>
            Spin up a guardrail agent with a name and a system prompt. You can refine scope,
            policies, and knowledge later.
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

            <div className="grid gap-2">
              <Label htmlFor="quick-agent-prompt">System prompt</Label>
              <Textarea
                id="quick-agent-prompt"
                value={systemPrompt}
                onChange={(event) => setSystemPrompt(event.target.value)}
                rows={6}
                className="font-mono leading-relaxed"
                required
              />
              <p className="text-xs text-muted-foreground">
                Minimum 20 characters. Describe the agent&apos;s purpose and the topics it must
                avoid or escalate.
              </p>
            </div>
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
