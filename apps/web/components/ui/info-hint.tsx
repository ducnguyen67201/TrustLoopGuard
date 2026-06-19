'use client';

import type { ReactNode } from 'react';
import { Info } from 'lucide-react';

import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { GLOSSARY, type GlossaryTerm } from '@/lib/glossary';
import { cn } from '@/lib/utils';

interface InfoHintProps {
  /** A glossary term key — its plain-language definition is shown on hover/focus. */
  term?: GlossaryTerm;
  /** Custom help text. Overrides `term` when both are given. */
  children?: ReactNode;
  /** Accessible label for the trigger button. */
  label?: string;
  /** Which side the bubble prefers to open on. */
  side?: 'top' | 'right' | 'bottom' | 'left';
  className?: string;
}

/**
 * A small "?" affordance that explains a domain term in plain language without
 * cluttering the surface. Sits next to a label, column header, or page title.
 * Keyboard- and touch-accessible (it is a real button); definitions come from
 * `lib/glossary.ts` so wording stays consistent everywhere. See
 * docs/concept/web-ui-conventions.md.
 */
export function InfoHint({ term, children, label, side = 'top', className }: InfoHintProps) {
  const content = children ?? (term ? GLOSSARY[term].short : null);
  if (!content) return null;

  const accessibleLabel =
    label ?? (term ? `What does “${GLOSSARY[term].label}” mean?` : 'More information');

  return (
    <TooltipProvider delayDuration={150}>
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            aria-label={accessibleLabel}
            className={cn(
              'inline-flex size-4 shrink-0 cursor-help items-center justify-center rounded-full align-middle text-muted-foreground/70 transition-colors hover:text-foreground focus-visible:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50',
              className,
            )}
          >
            <Info className="size-3.5" aria-hidden />
          </button>
        </TooltipTrigger>
        <TooltipContent side={side} className="max-w-xs text-left leading-relaxed [text-wrap:pretty]">
          {content}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
