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
  ...props
}: MarketingEventLinkProps) {
  return (
    <a
      {...props}
      onClick={(clickEvent) => {
        trackMarketingEvent(event, eventParams);
        onClick?.(clickEvent);
      }}
    >
      {children}
    </a>
  );
}
