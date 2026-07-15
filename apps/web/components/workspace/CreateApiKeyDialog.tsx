'use client';

import { IconAlertTriangle, IconCheck, IconCopy, IconKey } from '@tabler/icons-react';
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
  const [principal, setPrincipal] = useState('');
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
        body: JSON.stringify({
          name: name.trim(),
          environment_id: environmentId,
          ...(principal.trim() === '' ? {} : { principal_id: principal.trim() }),
        }),
      });
      const text = await res.text();
      if (!res.ok) {
        const detail = safeMessage(text);
        toast.error(
          detail
            ? `Couldn't create the key: ${detail}. Your details are still here — try again.`
            : "Couldn't create the key. Your details are still here — please try again.",
        );
        return;
      }
      const data = JSON.parse(text) as CreateApiKeyResponse;
      setCreated(data);
      setName('');
      setPrincipal('');
      router.refresh();
    } catch (err) {
      toast.error(
        err instanceof Error
          ? `Couldn't create the key: ${err.message}. Please check your connection and try again.`
          : "Couldn't create the key. Please check your connection and try again.",
      );
    } finally {
      setSubmitting(false);
    }
  }

  async function copyKey() {
    if (created === null) return;
    try {
      await navigator.clipboard.writeText(created.plaintext_key);
      setCopied(true);
      toast.success('Key copied — paste it into your app');
    } catch {
      toast.error('Copy failed. Select the key text and copy it manually before closing.');
    }
  }

  function onOpenChange(nextOpen: boolean) {
    setOpen(nextOpen);
    if (!nextOpen) {
      setCreated(null);
      setName('');
      setPrincipal('');
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
              <DialogTitle>Your key is ready — copy it now</DialogTitle>
              <DialogDescription>
                This is the only time you&apos;ll see the full secret. Copy it and paste it into
                your app. If you lose it, you can&apos;t get it back — just create a new key.
              </DialogDescription>
            </DialogHeader>

            <div className="grid gap-2">
              <Label htmlFor="created-api-key">Your secret key</Label>
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
                  aria-label="Copy your secret key"
                >
                  {copied ? <IconCheck /> : <IconCopy />}
                  {copied ? 'Copied' : 'Copy'}
                </Button>
              </div>
              {copied ? (
                <p className="flex items-center gap-1.5 text-xs text-[var(--color-permit)]">
                  <IconCheck className="size-3.5 shrink-0" aria-hidden />
                  Copied. Paste it into your app, then keep it somewhere safe.
                </p>
              ) : (
                <p className="text-xs text-muted-foreground">
                  Saved as{' '}
                  <span className="font-medium text-foreground">{created.api_key.name}</span>,
                  starting with{' '}
                  <span className="font-mono text-foreground">{created.api_key.prefix}…</span>
                </p>
              )}
            </div>

            <div className="flex items-start gap-2.5 rounded-lg border border-[var(--color-require-approval)]/40 bg-[var(--color-require-approval)]/10 px-3.5 py-3">
              <IconAlertTriangle
                className="mt-0.5 size-4 shrink-0 text-[var(--color-require-approval)]"
                aria-hidden
              />
              <p className="text-sm text-foreground">
                Treat this key like a password. Anyone who has it can connect as your app, so
                don&apos;t share it or paste it where others can see it.
              </p>
            </div>

            <DialogFooter>
              <Button type="button" onClick={() => onOpenChange(false)}>
                I&apos;ve copied it — done
              </Button>
            </DialogFooter>
          </div>
        ) : (
          <form onSubmit={onSubmit} className="grid gap-5">
            <DialogHeader>
              <DialogTitle>Create API key</DialogTitle>
              <DialogDescription>
                Create a key so your app can connect to the guardrail. You&apos;ll see the secret
                once, right after you create it — so have somewhere ready to paste it.
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
              <Label htmlFor="api-key-name">Name this key</Label>
              <Input
                id="api-key-name"
                required
                autoComplete="off"
                placeholder="Production SDK"
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                A name only you see, so you can recognize this key later — for example, the app or
                environment that uses it.
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
                This key only works for the environment you pick here.
              </p>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="api-key-principal">Principal (optional)</Label>
              <Input
                id="api-key-principal"
                autoComplete="off"
                placeholder="user:daniel"
                maxLength={256}
                value={principal}
                onChange={(event) => setPrincipal(event.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Bind this key to one person or agent. Spend caps, alerts, and the audit trail are
                then attributed to them. Leave blank for a workspace-level key.
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
