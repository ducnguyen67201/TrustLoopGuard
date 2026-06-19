import { AlertCircle, Loader2 } from 'lucide-react';

import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';

interface FormErrorProps {
  message: string | null;
}

/**
 * A designed, recoverable error panel. Lives in an assertive live region so a
 * screen reader announces failures as they appear, without stealing focus.
 */
export function FormError({ message }: FormErrorProps) {
  return (
    <div aria-live="assertive" role="alert" className="empty:hidden">
      {message ? (
        <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2.5 text-sm text-destructive">
          <AlertCircle aria-hidden className="mt-0.5 size-4 shrink-0" />
          <span>{message}</span>
        </div>
      ) : null}
    </div>
  );
}

/** An "or" divider between OAuth and credential sign-in. */
export function OrDivider() {
  return (
    <div className="flex items-center gap-3">
      <Separator className="flex-1" />
      <span className="font-mono text-[0.7rem] uppercase tracking-wider text-muted-foreground">
        or
      </span>
      <Separator className="flex-1" />
    </div>
  );
}

interface SpinnerProps {
  className?: string;
}

/** A reduced-motion-aware pending spinner for button labels. */
export function Spinner({ className }: SpinnerProps) {
  return (
    <Loader2
      aria-hidden
      className={cn('size-4 animate-spin motion-reduce:animate-none', className)}
    />
  );
}
