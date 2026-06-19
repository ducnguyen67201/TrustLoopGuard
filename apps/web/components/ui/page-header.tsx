import type { ReactNode } from 'react';

import { cn } from '@/lib/utils';

interface PageHeaderProps {
  /** Short context line above the title (e.g. workspace name). */
  eyebrow?: ReactNode;
  title: ReactNode;
  /** Sentence-length explanation shown below the title. */
  description?: ReactNode;
  /** Optional inline help next to the title — typically an `<InfoHint>`. */
  help?: ReactNode;
  /** Primary action(s), right-aligned on wide viewports. */
  actions?: ReactNode;
  className?: string;
}

/**
 * The one in-content page header used across every dashboard page. Gives every
 * surface the same rhythm: eyebrow → title (h1) → description, with the page's
 * primary action pinned to the right. See docs/concept/web-ui-conventions.md.
 */
export function PageHeader({
  eyebrow,
  title,
  description,
  help,
  actions,
  className,
}: PageHeaderProps) {
  return (
    <div
      className={cn(
        'flex flex-col gap-3 md:flex-row md:items-end md:justify-between',
        className,
      )}
    >
      <div className="grid min-w-0 gap-1">
        {eyebrow ? (
          <span className="truncate text-sm text-muted-foreground">{eyebrow}</span>
        ) : null}
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold tracking-tight text-balance text-foreground">
            {title}
          </h1>
          {help ? <span className="flex items-center">{help}</span> : null}
        </div>
        {description ? (
          <p className="max-w-prose text-sm text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
    </div>
  );
}
