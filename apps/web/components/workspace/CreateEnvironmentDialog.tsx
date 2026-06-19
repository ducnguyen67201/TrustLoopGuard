'use client';

import { IconStack2 } from '@tabler/icons-react';
import { type FormEvent, useState } from 'react';
import { toast } from 'sonner';

import { Dialog, DialogContent } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  DialogFooterBar,
  DialogShellHeader,
  FieldHint,
  FormRow,
} from '@/components/workspace/dialog-scaffold';

type CreatedEnvironment = {
  id: string;
  name: string;
  slug: string;
};

export function CreateEnvironmentDialog({
  open,
  onOpenChange,
  workspaceSlug,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspaceSlug: string;
  onCreated: (environment: CreatedEnvironment) => void;
}) {
  const [name, setName] = useState('');
  const [slug, setSlug] = useState('');
  const [description, setDescription] = useState('');
  const [slugEdited, setSlugEdited] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  function handleOpenChange(nextOpen: boolean) {
    onOpenChange(nextOpen);
    if (!nextOpen) {
      setName('');
      setSlug('');
      setDescription('');
      setSlugEdited(false);
      setSubmitting(false);
    }
  }

  function onNameChange(value: string) {
    setName(value);
    if (!slugEdited) {
      setSlug(slugifyEnvironment(value));
    }
  }

  function onSlugChange(value: string) {
    setSlugEdited(true);
    setSlug(slugifyEnvironment(value));
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;

    const trimmedName = name.trim();
    const trimmedSlug = slug.trim();
    if (trimmedName === '' || trimmedSlug === '') {
      toast.error('Name and slug are required');
      return;
    }

    setSubmitting(true);
    try {
      const res = await fetch(`/api/environments?workspace=${encodeURIComponent(workspaceSlug)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: trimmedName,
          slug: trimmedSlug,
          description: description.trim() || undefined,
        }),
      });
      const text = await res.text();
      if (!res.ok) {
        toast.error(safeMessage(text) ?? `create environment failed (${res.status})`);
        return;
      }

      const environment = JSON.parse(text) as CreatedEnvironment;
      toast.success(`Created environment "${environment.name}"`);
      handleOpenChange(false);
      onCreated(environment);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'create environment failed');
    } finally {
      setSubmitting(false);
    }
  }

  const canSubmit = name.trim() !== '' && slug.trim() !== '';

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <form onSubmit={onSubmit} className="grid gap-5">
          <DialogShellHeader
            icon={<IconStack2 />}
            eyebrow="New environment"
            title="Create an environment"
            description="A separate lane for policy deployments and runtime keys — for example QA, staging, or production."
          />
          <fieldset disabled={submitting} className="grid gap-4">
            <FormRow>
              <Label htmlFor="environment-name">Name</Label>
              <Input
                id="environment-name"
                required
                autoComplete="off"
                placeholder="QA"
                value={name}
                onChange={(event) => onNameChange(event.target.value)}
              />
            </FormRow>
            <FormRow>
              <Label htmlFor="environment-slug">Slug</Label>
              <Input
                id="environment-slug"
                required
                autoComplete="off"
                placeholder="qa"
                value={slug}
                onChange={(event) => onSlugChange(event.target.value)}
                className="font-mono"
              />
              <FieldHint>Used in URLs and runtime keys. Lowercase letters, numbers, and dashes.</FieldHint>
            </FormRow>
            <FormRow>
              <Label htmlFor="environment-description">
                Description <span className="text-muted-foreground">(optional)</span>
              </Label>
              <Input
                id="environment-description"
                autoComplete="off"
                placeholder="What this environment is for"
                value={description}
                onChange={(event) => setDescription(event.target.value)}
              />
            </FormRow>
          </fieldset>
          <DialogFooterBar
            onCancel={() => handleOpenChange(false)}
            submitting={submitting}
            submitDisabled={!canSubmit}
            submitLabel="Create environment"
            submittingLabel="Creating…"
          />
        </form>
      </DialogContent>
    </Dialog>
  );
}

function slugifyEnvironment(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, '-')
    .replace(/-{2,}/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 64);
}

function safeMessage(text: string): string | null {
  try {
    const parsed = JSON.parse(text) as { message?: string; error?: string };
    return parsed.message ?? parsed.error ?? null;
  } catch {
    return text.length > 0 ? text : null;
  }
}
