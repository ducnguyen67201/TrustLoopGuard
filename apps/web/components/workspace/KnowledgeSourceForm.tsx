'use client';

import type { ReactNode } from 'react';
import { useId, useState } from 'react';
import { useFormStatus } from 'react-dom';
import {
  IconFileText,
  IconLink,
  IconLoader2,
  IconNote,
  IconUpload,
} from '@tabler/icons-react';
import type { Icon } from '@tabler/icons-react';

import { createKnowledgeSource } from '@/app/knowledge-sources/actions';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';

type SourceKind = 'url' | 'file' | 'note';

const KIND_OPTIONS: {
  value: SourceKind;
  label: string;
  hint: string;
  icon: Icon;
}[] = [
  { value: 'url', label: 'URL', hint: 'Link a hosted page', icon: IconLink },
  { value: 'file', label: 'File', hint: 'Upload a document', icon: IconUpload },
  { value: 'note', label: 'Note', hint: 'Paste text inline', icon: IconNote },
];

const MAX_FILE_MB = 10;

interface KnowledgeSourceFormProps {
  workspaceSlug: string;
  /** Default Cancel target. Ignored when `cancelSlot` is provided. */
  cancelHref: string;
  /**
   * Overrides the default anchor Cancel — the dialog passes a `DialogClose`
   * button so cancelling closes the modal instead of navigating.
   */
  cancelSlot?: ReactNode;
  /** Tighter spacing for the dialog surface. */
  variant?: 'page' | 'dialog';
}

/**
 * Kind-aware create form shared by the full-page route and the create dialog so
 * both surfaces read identically. The selected kind drives which "location"
 * control is shown (URL field, file upload, or inline note), preserving the
 * exact `createKnowledgeSource` server-action contract and field names.
 */
export function KnowledgeSourceForm({
  workspaceSlug,
  cancelHref,
  cancelSlot,
  variant = 'page',
}: KnowledgeSourceFormProps) {
  const [kind, setKind] = useState<SourceKind>('url');
  const titleId = useId();
  const locationId = useId();
  const fileId = useId();
  const notesId = useId();

  const isDialog = variant === 'dialog';

  return (
    <form
      action={createKnowledgeSource}
      className="grid gap-6"
      encType="multipart/form-data"
    >
      <input type="hidden" name="workspaceSlug" value={workspaceSlug} />
      {/* Radix Select posts via JS only; a native hidden input keeps `kind` in FormData. */}
      <input type="hidden" name="kind" value={kind} />

      <FieldGroup
        label="Title"
        htmlFor={titleId}
        hint="A short, human name. Shown in the knowledge table and decision traces."
      >
        <Input
          id={titleId}
          name="title"
          placeholder="Refund policy"
          autoComplete="off"
          required
        />
      </FieldGroup>

      <fieldset className="grid gap-2">
        <legend className="mb-2 text-sm font-medium leading-none">Source type</legend>
        <div className="grid gap-2 sm:grid-cols-3" role="radiogroup" aria-label="Source type">
          {KIND_OPTIONS.map((option) => {
            const selected = kind === option.value;
            const OptionIcon = option.icon;
            return (
              <button
                key={option.value}
                type="button"
                role="radio"
                aria-checked={selected}
                onClick={() => setKind(option.value)}
                className={cn(
                  'group flex items-start gap-3 rounded-md border bg-card p-3 text-left transition-[color,box-shadow,border-color]',
                  'outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50',
                  selected
                    ? 'border-primary ring-[3px] ring-primary/20'
                    : 'border-input hover:border-ring/60 hover:bg-accent/40',
                )}
              >
                <OptionIcon
                  className={cn(
                    'mt-0.5 size-4 shrink-0',
                    selected ? 'text-primary' : 'text-muted-foreground',
                  )}
                  aria-hidden
                />
                <span className="grid gap-0.5">
                  <span className="text-sm font-medium leading-none">{option.label}</span>
                  <span className="text-xs text-muted-foreground">{option.hint}</span>
                </span>
              </button>
            );
          })}
        </div>
      </fieldset>

      {kind === 'url' ? (
        <FieldGroup
          label="Location"
          htmlFor={locationId}
          hint="The full URL the engine can fetch for context."
        >
          <Input
            id={locationId}
            name="location"
            type="url"
            inputMode="url"
            className="font-mono text-sm"
            placeholder="https://example.com/help/refunds"
            autoComplete="off"
            required
          />
        </FieldGroup>
      ) : null}

      {kind === 'file' ? (
        <FieldGroup
          label="File"
          htmlFor={fileId}
          hint={`Required for file sources. Up to ${MAX_FILE_MB} MB.`}
        >
          <label
            htmlFor={fileId}
            className={cn(
              'flex cursor-pointer flex-col items-center gap-2 rounded-md border border-dashed border-input bg-muted/30 px-4 py-6 text-center transition-colors',
              'hover:border-ring/60 hover:bg-accent/40 focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50',
            )}
          >
            <IconUpload className="size-5 text-muted-foreground" aria-hidden />
            <span className="text-sm font-medium">Choose a file to upload</span>
            <span className="text-xs text-muted-foreground">
              PDF, Markdown, or plain text work best
            </span>
            <Input
              id={fileId}
              name="file"
              type="file"
              required
              className="sr-only"
            />
          </label>
        </FieldGroup>
      ) : null}

      <FieldGroup
        label={kind === 'note' ? 'Note' : 'Notes'}
        htmlFor={notesId}
        hint={
          kind === 'note'
            ? 'The text the engine grounds on. This is the source content for a note.'
            : 'Optional context for your team about this source.'
        }
      >
        <Textarea
          id={notesId}
          name="notes"
          className={kind === 'note' ? 'min-h-32' : undefined}
          placeholder={
            kind === 'note'
              ? 'Paste the approved policy text or guidance here…'
              : 'What should the team know about this source?'
          }
          required={kind === 'note'}
        />
      </FieldGroup>

      <p className="rounded-md border bg-muted/30 px-3 py-2.5 text-xs text-muted-foreground">
        Source records and uploaded file content are stored in the workspace. Retrieval indexing can
        be attached later.
      </p>

      <div
        className={cn(
          'flex flex-col-reverse gap-2 sm:flex-row sm:justify-end',
          isDialog ? 'pt-1' : 'border-t pt-5',
        )}
      >
        {cancelSlot ?? (
          <Button variant="outline" type="button" asChild>
            <a href={cancelHref}>Cancel</a>
          </Button>
        )}
        <SubmitButton />
      </div>
    </form>
  );
}

function SubmitButton() {
  const { pending } = useFormStatus();
  return (
    <Button type="submit" disabled={pending} aria-disabled={pending}>
      {pending ? (
        <>
          <IconLoader2 className="size-4 animate-spin motion-reduce:animate-none" aria-hidden />
          Adding…
        </>
      ) : (
        <>
          <IconFileText className="size-4" aria-hidden />
          Add source
        </>
      )}
    </Button>
  );
}

function FieldGroup({
  label,
  htmlFor,
  hint,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: string;
  children: React.ReactNode;
}) {
  const hintId = hint ? `${htmlFor}-hint` : undefined;
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
      {hint ? (
        <p id={hintId} className="text-xs text-muted-foreground">
          {hint}
        </p>
      ) : null}
    </div>
  );
}
