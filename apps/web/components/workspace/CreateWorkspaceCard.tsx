'use client';

import { IconBuildingCommunity } from '@tabler/icons-react';
import { Loader2 } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useState, type FormEvent } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface CreatedWorkspace {
  id: string;
  slug: string;
  name: string;
}

export function CreateWorkspaceCard() {
  const router = useRouter();
  const [name, setName] = useState('');
  const [submitting, setSubmitting] = useState(false);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    const trimmed = name.trim();
    if (trimmed === '') return;
    setSubmitting(true);
    try {
      const res = await fetch('/api/me/workspaces', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: trimmed }),
      });
      const text = await res.text();
      if (!res.ok) {
        toast.error(safeMessage(text) ?? `Could not create workspace (${res.status})`);
        return;
      }
      const ws = JSON.parse(text) as CreatedWorkspace;
      toast.success(`Workspace “${ws.name}” created`);
      router.replace(`/?workspace=${encodeURIComponent(ws.slug)}`);
      router.refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Could not create workspace';
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  const canSubmit = name.trim() !== '';

  return (
    <Card>
      <CardHeader>
        <CardDescription className="flex items-center gap-1.5">
          <IconBuildingCommunity className="size-3.5" />
          Or start your own
        </CardDescription>
        <CardTitle className="text-base">Create a workspace</CardTitle>
        <CardDescription>
          You become its owner and can invite teammates. You can still be invited
          to other workspaces later.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={onSubmit} className="grid gap-4">
          <div className="grid gap-2">
            <Label htmlFor="workspace-name">Workspace name</Label>
            <Input
              id="workspace-name"
              type="text"
              required
              maxLength={64}
              autoComplete="off"
              placeholder="e.g. Acme Support"
              value={name}
              onChange={(e) => setName(e.target.value)}
              disabled={submitting}
            />
          </div>
          <div className="flex justify-end border-t pt-4">
            <Button type="submit" disabled={submitting || !canSubmit}>
              {submitting ? (
                <>
                  <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
                  Creating…
                </>
              ) : (
                'Create workspace'
              )}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
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
