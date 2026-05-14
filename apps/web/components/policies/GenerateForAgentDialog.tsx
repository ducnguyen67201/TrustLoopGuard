'use client';

import { Loader2 } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';
import type { PolicyDocument } from '@trustloopguard/sdk';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { generateGuardrails } from '@/lib/guardrails';

interface GenerateForAgentDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Called after a successful generate so the parent list can refresh. */
  onGenerated?: () => void;
}

export function GenerateForAgentDialog({
  open,
  onOpenChange,
  onGenerated,
}: GenerateForAgentDialogProps) {
  const [agentId, setAgentId] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [generated, setGenerated] = useState<PolicyDocument[] | null>(null);

  function reset() {
    setAgentId('');
    setGenerated(null);
    setSubmitting(false);
  }

  function close(open: boolean) {
    if (!open) reset();
    onOpenChange(open);
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const trimmed = agentId.trim();
    if (trimmed === '') return;

    setSubmitting(true);
    try {
      const response = await generateGuardrails(trimmed);
      setGenerated(response.generated);
      toast.success(
        response.generated.length === 0
          ? 'No guardrails generated'
          : `Generated ${response.generated.length} guardrail${response.generated.length === 1 ? '' : 's'} (disabled).`,
      );
      onGenerated?.();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'unknown error';
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Auto-generate guardrails for an agent</DialogTitle>
          <DialogDescription>
            Reads the agent&apos;s stored <code>system_prompt</code> and derives a
            tailored guardrail set. Every generated policy is saved disabled —
            review and enable them individually from the policies list.
          </DialogDescription>
        </DialogHeader>

        {generated === null ? (
          <form onSubmit={submit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="agent-id">Agent ID</Label>
              <Input
                id="agent-id"
                value={agentId}
                onChange={(e) => setAgentId(e.target.value)}
                placeholder="baker-9000"
                disabled={submitting}
                autoFocus
              />
              <p className="text-muted-foreground text-xs">
                The agent must already be registered with a{' '}
                <code>system_prompt</code>.
              </p>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="ghost"
                onClick={() => close(false)}
                disabled={submitting}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={submitting || agentId.trim() === ''}>
                {submitting ? (
                  <>
                    <Loader2 className="mr-2 size-4 animate-spin" />
                    Generating…
                  </>
                ) : (
                  'Generate'
                )}
              </Button>
            </DialogFooter>
          </form>
        ) : (
          <div className="space-y-4">
            {generated.length === 0 ? (
              <p className="text-muted-foreground text-sm">
                The model returned no guardrails for this agent.
              </p>
            ) : (
              <ul className="divide-y rounded-md border">
                {generated.map((doc) => (
                  <li key={doc.id} className="flex items-start gap-3 p-3">
                    <Badge variant="outline" className="mt-0.5 shrink-0">
                      {doc.severity}
                    </Badge>
                    <div className="min-w-0 flex-1">
                      <p className="truncate font-mono text-sm font-medium">
                        {doc.id}
                      </p>
                      {doc.description !== undefined ? (
                        <p className="text-muted-foreground mt-1 text-sm">
                          {doc.description}
                        </p>
                      ) : null}
                    </div>
                  </li>
                ))}
              </ul>
            )}
            <p className="text-muted-foreground text-xs">
              These policies are saved with <code>enabled=false</code>. Flip the
              toggle on the policies list to put them into effect for{' '}
              <code>{agentId}</code>.
            </p>
            <DialogFooter>
              <Button onClick={() => close(false)}>Done</Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
