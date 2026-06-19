'use client';

import type { FormEvent, ReactNode } from 'react';
import { useId, useState } from 'react';
import {
  IconAlertTriangle,
  IconFileText,
  IconLink,
  IconLoader2,
  IconNote,
  IconUpload,
} from '@tabler/icons-react';
import type { Icon } from '@tabler/icons-react';

import { createKnowledgeSource } from '@/app/knowledge-sources/actions';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { InfoHint } from '@/components/ui/info-hint';
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
  { value: 'file', label: 'A file', hint: 'Upload a PDF or document', icon: IconUpload },
  { value: 'url', label: 'A web link', hint: 'Point to a page online', icon: IconLink },
  { value: 'note', label: 'Pasted text', hint: 'Type or paste it in', icon: IconNote },
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
 *
 * Copy is written for a non-technical owner: friendly type names, a required/
 * optional marker on every field, a plain hint, and gentle (non-blocking)
 * feedback when a web link does not look like a web address. The validation
 * that actually runs lives in the server action — this only nudges.
 */
export function KnowledgeSourceForm({
  workspaceSlug,
  cancelHref,
  cancelSlot,
  variant = 'page',
}: KnowledgeSourceFormProps) {
  const [kind, setKind] = useState<SourceKind>('file');
  const [locationValue, setLocationValue] = useState('');
  const [locationTouched, setLocationTouched] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const titleId = useId();
  const locationId = useId();
  const fileId = useId();
  const notesId = useId();
  const formErrorId = useId();

  const isDialog = variant === 'dialog';

  // The server action stays the source of truth and redirects on success.
  // We call it inside a client try/catch so a server-side failure (bad file,
  // bad web address) lands as a friendly inline message instead of resetting
  // the submit button with no visible feedback. A successful redirect resolves
  // normally — it never rejects here — so the happy path is untouched.
  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSubmitting) return;
    setSubmitError(null);
    setIsSubmitting(true);
    try {
      await createKnowledgeSource(new FormData(event.currentTarget));
    } catch (error: unknown) {
      setSubmitError(toFriendlyError(error));
      setIsSubmitting(false);
    }
  }

  // A purely advisory check so we can show a friendly nudge. It never blocks
  // submit — the server action stays the single source of truth for validation.
  const trimmedLocation = locationValue.trim();
  const looksLikeWebAddress =
    /^https?:\/\/\S+\.\S+/i.test(trimmedLocation) || /^\S+\.\S+/.test(trimmedLocation);
  const showLocationNudge =
    kind === 'url' && locationTouched && trimmedLocation.length > 0 && !looksLikeWebAddress;

  return (
    <form
      onSubmit={handleSubmit}
      className="grid gap-6"
      encType="multipart/form-data"
      aria-describedby={submitError ? formErrorId : undefined}
    >
      <input type="hidden" name="workspaceSlug" value={workspaceSlug} />
      {/* Radix-style buttons set `kind` via JS only; a native hidden input keeps it in FormData. */}
      <input type="hidden" name="kind" value={kind} />

      {submitError ? (
        <Alert id={formErrorId} variant="destructive" aria-live="assertive">
          <IconAlertTriangle aria-hidden />
          <AlertTitle>We couldn&apos;t add this source</AlertTitle>
          <AlertDescription>{submitError}</AlertDescription>
        </Alert>
      ) : null}

      <p className="flex items-start gap-1.5 text-sm text-muted-foreground">
        <span>
          A{' '}
          <span className="font-medium text-foreground">knowledge source</span> is approved content —
          a file, a web link, or pasted text — the guardrail can trust and check answers against.
        </span>
        <InfoHint side="bottom" label="What is a knowledge source?">
          Approved content you give the guardrail so it can ground its answers in material you trust.
        </InfoHint>
      </p>

      <FieldGroup
        label="Name"
        htmlFor={titleId}
        required
        hint="A short name you'll recognise later — like “Refund policy”. It shows up in your list and in decision records."
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
        <legend className="mb-2 flex items-center gap-1.5 text-sm font-medium leading-none">
          What are you adding?
          <RequiredMark />
        </legend>
        <div className="grid gap-2 sm:grid-cols-3" role="radiogroup" aria-label="What are you adding?">
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
          label="Web address"
          htmlFor={locationId}
          required
          hint="Paste the full link to the page, starting with https://"
          error={showLocationNudge ? "That doesn't look like a web address — it usually starts with https://" : undefined}
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
            value={locationValue}
            onChange={(event) => setLocationValue(event.target.value)}
            onBlur={() => setLocationTouched(true)}
            aria-invalid={showLocationNudge || undefined}
          />
        </FieldGroup>
      ) : null}

      {kind === 'file' ? (
        <div className="grid gap-2">
          <span className="flex items-center gap-1.5 text-sm font-medium">
            File
            <RequiredMark />
          </span>
          {/* The dropzone is the single <label> for the input, so the file field
              is not double-labelled. */}
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
              PDF, Markdown, or plain text work best — up to {MAX_FILE_MB} MB
            </span>
            <Input
              id={fileId}
              name="file"
              type="file"
              required
              className="sr-only"
            />
          </label>
        </div>
      ) : null}

      <FieldGroup
        label={kind === 'note' ? 'Your text' : 'Notes'}
        htmlFor={notesId}
        required={kind === 'note'}
        hint={
          kind === 'note'
            ? 'Paste the approved wording here. This is the trusted content the guardrail will check answers against.'
            : 'A quick note for your team about what this is — totally optional.'
        }
      >
        <Textarea
          id={notesId}
          name="notes"
          className={kind === 'note' ? 'min-h-32' : undefined}
          placeholder={
            kind === 'note'
              ? 'Paste the approved policy text or guidance here…'
              : 'e.g. “Official wording — keep in sync with the help centre.”'
          }
          required={kind === 'note'}
        />
      </FieldGroup>

      <p className="rounded-md border bg-muted/30 px-3 py-2.5 text-xs text-muted-foreground">
        We save what you add to this workspace. Once it&apos;s read and stored, the guardrail can start
        checking answers against it.
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
        <SubmitButton pending={isSubmitting} />
      </div>
    </form>
  );
}

function SubmitButton({ pending }: { pending: boolean }) {
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

/**
 * Maps a thrown server-action failure to plain, fix-it copy a non-technical
 * owner can act on. Next masks raw messages in production, so unknown causes
 * fall back to a friendly generic line rather than leaking internals or
 * showing nothing.
 */
function toFriendlyError(error: unknown): string {
  const message = error instanceof Error ? error.message.toLowerCase() : '';
  if (message.includes('10 mb') || message.includes('smaller')) {
    return 'That file is over the 10 MB limit. Try a smaller file, or split it up.';
  }
  if (message.includes('file is required')) {
    return 'Please choose a file to upload before adding this source.';
  }
  if (message.includes('title')) {
    return 'Please give this source a name so you can find it later.';
  }
  if (message.includes('location') || message.includes('url') || message.includes('address')) {
    return "That web address doesn't look right. Paste the full link, starting with https://.";
  }
  return "We couldn't save this source. Check your file or web address and try again.";
}

function RequiredMark() {
  return (
    <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground" aria-hidden>
      Required
    </span>
  );
}

function OptionalMark() {
  return (
    <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground/70" aria-hidden>
      Optional
    </span>
  );
}

function FieldGroup({
  label,
  htmlFor,
  hint,
  error,
  required = false,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: string;
  error?: string | undefined;
  required?: boolean;
  children: React.ReactNode;
}) {
  const hintId = hint ? `${htmlFor}-hint` : undefined;
  const errorId = error ? `${htmlFor}-error` : undefined;
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor} className="flex items-center gap-2">
        <span>{label}</span>
        {required ? <RequiredMark /> : <OptionalMark />}
      </Label>
      {children}
      {error ? (
        <p id={errorId} role="alert" className="text-xs font-medium text-destructive">
          {error}
        </p>
      ) : null}
      {hint ? (
        <p id={hintId} className="text-xs text-muted-foreground">
          {hint}
        </p>
      ) : null}
    </div>
  );
}
