import posthog from 'posthog-js';
import { capturePostHogMarketingEvent, type PostHogBrowserClient } from './posthog';

export type MarketingEventName =
  | 'install_sdk_click'
  | 'book_meeting_click'
  | 'docs_click'
  | 'github_click'
  | 'waitlist_submit'
  | 'landing_cta_click'
  | 'demo_click'
  | 'demo_started'
  | 'demo_decision_shown'
  | 'demo_policy_changed'
  | 'healthcare_demo_started'
  | 'healthcare_demo_decision_shown'
  | 'contextual_demo_started'
  | 'contextual_demo_decision_shown';

export interface MarketingEventParams extends Record<string, string | undefined> {
  page?: string;
  location?: string;
  label?: string;
  scenario?: string;
  decision?: string;
  outcome?: string;
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
