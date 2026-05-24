'use client';

import { IconCopy, IconKey } from '@tabler/icons-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { useState, type FormEvent } from 'react';
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
    toast.success('API key copied');
  }

  function onOpenChange(nextOpen: boolean) {
    setOpen(nextOpen);
    if (!nextOpen) {
      setCreated(null);
      setName('');
      setEnvironmentId(activeEnvironmentId);
      setSubmitting(false);
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
          <div className="grid gap-4">
            <DialogHeader>
              <DialogTitle>API key created</DialogTitle>
              <DialogDescription>Store this key now. It will not be shown again.</DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
              <Label htmlFor="created-api-key">Bearer key</Label>
              <div className="flex gap-2">
                <Input
                  id="created-api-key"
                  readOnly
                  value={created.plaintext_key}
                  className="font-mono text-xs"
                />
                <Button type="button" variant="outline" size="icon" onClick={copyKey}>
                  <IconCopy />
                  <span className="sr-only">Copy API key</span>
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                Prefix shown in the table: {created.api_key.prefix}
              </p>
            </div>
            <DialogFooter>
              <Button type="button" onClick={() => onOpenChange(false)}>
                Done
              </Button>
            </DialogFooter>
          </div>
        ) : (
          <form onSubmit={onSubmit} className="grid gap-4">
            <DialogHeader>
              <DialogTitle>Create API key</DialogTitle>
            <DialogDescription>
                Issue an environment-scoped key for SDK runtime checks.
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
                      {environment.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <DialogFooter>
              <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={submitting || name.trim() === ''}>
                {submitting ? 'Creating...' : 'Create key'}
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
