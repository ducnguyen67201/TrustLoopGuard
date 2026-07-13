import posthog from 'posthog-js';
import { capturePostHogMarketingEvent, type PostHogBrowserClient } from './posthog';

export type MarketingEventName =
  | 'install_sdk_click'
  | 'book_meeting_click'
  | 'docs_click'
  | 'github_click'
  | 'waitlist_submit'
  | 'landing_cta_click';

export interface MarketingEventParams extends Record<string, string | undefined> {
  page?: string;
  location?: string;
  label?: string;
}

declare global {
  interface Window {
    dataLayer?: Array<Record<string, unknown>>;
  }
}

export function trackMarketingEvent(
  event: MarketingEventName,
  params: MarketingEventParams = {},
  postHogClient: PostHogBrowserClient = posthog,
): void {
  if (typeof window === 'undefined') return;

  window.dataLayer = window.dataLayer ?? [];
  window.dataLayer.push({
    event,
    ...params,
  });
  capturePostHogMarketingEvent(postHogClient, event, params);
}
