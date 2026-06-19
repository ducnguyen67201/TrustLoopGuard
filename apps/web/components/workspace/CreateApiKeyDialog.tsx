'use client';

import {
  IconAlertTriangle,
  IconCheck,
  IconCopy,
  IconKey,
} from '@tabler/icons-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { useEffect, useState, type FormEvent } from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { WorkspaceEnvironmentSummary } from '@/lib/server/dashboard-data';

type CreateApiKeyResponse = {
  api_key: {
    id: string;
    name: string;
    prefix: string;
  };
  plaintext_key: string;
};

export function CreateApiKeyDialog({
  environments,
  activeEnvironmentId,
}: {
  environments: WorkspaceEnvironmentSummary[];
  activeEnvironmentId: string;
}) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const workspace = searchParams.get('workspace') ?? '';

  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [environmentId, setEnvironmentId] = useState(activeEnvironmentId);
  const [submitting, setSubmitting] = useState(false);
  const [created, setCreated] = useState<CreateApiKeyResponse | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    setEnvironmentId(activeEnvironmentId);
  }, [activeEnvironmentId]);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);

    const queryString = workspace ? `?workspace=${encodeURIComponent(workspace)}` : '';
    try {
      const res = await fetch(`/api/api-keys${queryString}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: name.trim(), environment_id: environmentId }),
      });
      const text = await res.text();
      if (!res.ok) {
        toast.error(safeMessage(text) ?? `create key failed (${res.status})`);
        return;
      }
      const data = JSON.parse(text) as CreateApiKeyResponse;
      setCreated(data);
      setName('');
      router.refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'create key failed');
    } finally {
      setSubmitting(false);
    }
  }

  async function copyKey() {
    if (created === null) return;
    await navigator.clipboard.writeText(created.plaintext_key);
    setCopied(true);
    toast.success('API key copied');
  }

  function onOpenChange(nextOpen: boolean) {
    setOpen(nextOpen);
    if (!nextOpen) {
      setCreated(null);
      setName('');
      setEnvironmentId(activeEnvironmentId);
      setSubmitting(false);
      setCopied(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogTrigger asChild>
        <Button>
          <IconKey />
          Create key
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-lg">
        {created ? (
          <div className="grid gap-5">
            <DialogHeader>
              <DialogTitle>API key created</DialogTitle>
              <DialogDescription>
                Copy this secret now — it is shown once and cannot be retrieved again.
              </DialogDescription>
            </DialogHeader>

            <div className="flex items-start gap-2.5 rounded-lg border border-[var(--color-escalate)]/40 bg-[var(--color-escalate)]/10 px-3.5 py-3">
              <IconAlertTriangle
                className="mt-0.5 size-4 shrink-0 text-[var(--color-escalate)]"
                aria-hidden
              />
              <p className="text-sm text-foreground">
                Store the key in your secret manager before closing. Anyone with this value can
                authenticate as <span className="font-mono text-xs">{created.api_key.name}</span>.
              </p>
            </div>

            <div className="grid gap-2">
              <Label htmlFor="created-api-key">Bearer key</Label>
              <div className="flex gap-2">
                <Input
                  id="created-api-key"
                  readOnly
                  value={created.plaintext_key}
                  onFocus={(event) => event.currentTarget.select()}
                  className="font-mono text-xs"
                />
                <Button
                  type="button"
                  variant={copied ? 'outline' : 'default'}
                  onClick={copyKey}
                  aria-label="Copy API key"
                >
                  {copied ? <IconCheck /> : <IconCopy />}
                  {copied ? 'Copied' : 'Copy'}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                Table prefix:{' '}
                <span className="font-mono text-foreground">{created.api_key.prefix}…</span>
              </p>
            </div>

            <DialogFooter>
              <Button type="button" onClick={() => onOpenChange(false)}>
                Done
              </Button>
            </DialogFooter>
          </div>
        ) : (
          <form onSubmit={onSubmit} className="grid gap-5">
            <DialogHeader>
              <DialogTitle>Create API key</DialogTitle>
              <DialogDescription>
                Issue an environment-scoped credential for SDK runtime checks. The full key is
                revealed only once, right after you create it.
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
              <Label htmlFor="api-key-name">Name</Label>
              <Input
                id="api-key-name"
                required
                autoComplete="off"
                placeholder="Production SDK"
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                A label to recognize this key later, e.g. the service or environment that uses it.
              </p>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="api-key-environment">Environment</Label>
              <Select value={environmentId} onValueChange={setEnvironmentId}>
                <SelectTrigger id="api-key-environment" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {environments.map((environment) => (
                    <SelectItem key={environment.id} value={environment.id}>
                      <span className="flex items-center gap-2">
                        {environment.name}
                        {environment.isDefault ? (
                          <Badge variant="secondary" className="text-[0.625rem]">
                            default
                          </Badge>
                        ) : null}
                      </span>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                The key authenticates checks for this environment only.
              </p>
            </div>
            <DialogFooter>
              <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={submitting || name.trim() === ''}>
                {submitting ? 'Creating…' : 'Create key'}
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  );
}

function safeMessage(text: string): string | null {
  try {
    const parsed = JSON.parse(text) as { message?: string; error?: string };
    return parsed.message ?? parsed.error ?? null;
  } catch {
    return text.length > 0 ? text : null;
  }
}
