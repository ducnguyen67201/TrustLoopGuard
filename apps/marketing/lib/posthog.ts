import type { MarketingEventName, MarketingEventParams } from './gtm';

export interface PostHogBrowserClient {
  __loaded: boolean;
  init(
    token: string,
    options: {
      api_host: string;
      defaults: '2026-05-30';
    },
  ): void;
  register(properties: Record<string, string>): void;
  capture(event: string, properties?: MarketingEventParams): void;
}

interface PostHogConfig {
  projectToken: string | undefined;
  host: string;
}

export function initializeMarketingPostHog(
  client: PostHogBrowserClient,
  config: PostHogConfig,
): boolean {
  if (!config.projectToken) return false;

  client.init(config.projectToken, {
    api_host: config.host,
    defaults: '2026-05-30',
  });
  client.register({ app_surface: 'marketing' });
  return true;
}

export function capturePostHogMarketingEvent(
  client: PostHogBrowserClient,
  event: MarketingEventName,
  params: MarketingEventParams,
): void {
  if (!client.__loaded) return;

  client.capture(event, params);
}
