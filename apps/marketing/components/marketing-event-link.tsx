'use client';

import type { AnchorHTMLAttributes, ReactNode } from 'react';
import {
  trackMarketingEvent,
  type MarketingEventName,
  type MarketingEventParams,
} from '@/lib/gtm';

interface MarketingEventLinkProps extends AnchorHTMLAttributes<HTMLAnchorElement> {
  event: MarketingEventName;
  eventParams?: MarketingEventParams;
  children: ReactNode;
}

export function MarketingEventLink({
  event,
  eventParams,
  onClick,
  children,
  rel,
  target,
  ...props
}: MarketingEventLinkProps) {
  const safeRel = target === '_blank' ? mergeRel(rel, 'noopener noreferrer') : rel;

  return (
    <a
      {...props}
      rel={safeRel}
      target={target}
      onClick={(clickEvent) => {
        trackMarketingEvent(event, eventParams);
        onClick?.(clickEvent);
      }}
    >
      {children}
    </a>
  );
}

function mergeRel(rel: string | undefined, required: string): string {
  const tokens = new Set(`${rel ?? ''} ${required}`.trim().split(/\s+/).filter(Boolean));
  return Array.from(tokens).join(' ');
}
