'use client';

import { Loader2 } from 'lucide-react';
import type { ComponentProps, ReactNode } from 'react';

import { Button } from '@/components/ui/button';
import {
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { cn } from '@/lib/utils';

/**
 * Shared presentational scaffold so every workspace create/invite dialog reads
 * with one rhythm: an icon eyebrow + title + description header, a body, and a
 * footer that places Cancel left and the primary action right with a built-in
 * loading state. Logic stays in the calling dialog — this owns layout only.
 */

interface DialogShellHeaderProps {
  icon: ReactNode;
  eyebrow: string;
  title: ReactNode;
  description: ReactNode;
}

export function DialogShellHeader({
  icon,
  eyebrow,
  title,
  description,
}: DialogShellHeaderProps) {
  return (
    <DialogHeader className="space-y-3 text-left">
      <div className="flex items-center gap-3">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-md border bg-muted text-muted-foreground [&_svg]:size-4">
          {icon}
        </span>
        <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          {eyebrow}
        </span>
      </div>
      <div className="space-y-1.5">
        <DialogTitle className="text-base">{title}</DialogTitle>
        <DialogDescription className="leading-relaxed">{description}</DialogDescription>
      </div>
    </DialogHeader>
  );
}

/** A single labelled vertical field stack — the canonical form-row spacing. */
export function FormRow({ children }: { children: ReactNode }) {
  return <div className="grid gap-2">{children}</div>;
}

/** Small hint/validation line under a field. Switches to destructive when invalid. */
export function FieldHint({
  children,
  invalid = false,
}: {
  children: ReactNode;
  invalid?: boolean;
}) {
  return (
    <p className={cn('text-xs', invalid ? 'text-destructive' : 'text-muted-foreground')}>
      {children}
    </p>
  );
}

interface DialogFooterBarProps {
  onCancel: () => void;
  cancelDisabled?: boolean;
  submitDisabled?: boolean;
  submitting: boolean;
  submitLabel: string;
  submittingLabel: string;
  submitVariant?: ComponentProps<typeof Button>['variant'];
}

export function DialogFooterBar({
  onCancel,
  cancelDisabled = false,
  submitDisabled = false,
  submitting,
  submitLabel,
  submittingLabel,
  submitVariant,
}: DialogFooterBarProps) {
  return (
    <DialogFooter className="mt-1 gap-2 border-t pt-4">
      <Button type="button" variant="ghost" onClick={onCancel} disabled={cancelDisabled}>
        Cancel
      </Button>
      <Button type="submit" variant={submitVariant} disabled={submitDisabled || submitting}>
        {submitting ? (
          <>
            <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
            {submittingLabel}
          </>
        ) : (
          submitLabel
        )}
      </Button>
    </DialogFooter>
  );
}
