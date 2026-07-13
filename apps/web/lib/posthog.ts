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
  identify(distinctId: string, properties: { name: string; email: string }): void;
  get_distinct_id(): string;
  reset(): void;
}

interface PostHogConfig {
  projectToken: string | undefined;
  host: string;
}

export interface PostHogUser {
  id: string;
  name: string;
  email: string;
}

export function initializeDashboardPostHog(
  client: PostHogBrowserClient,
  config: PostHogConfig,
): boolean {
  if (!config.projectToken) return false;

  client.init(config.projectToken, {
    api_host: config.host,
    defaults: '2026-05-30',
  });
  client.register({ app_surface: 'dashboard' });
  return true;
}

export function identifyPostHogUser(client: PostHogBrowserClient, user: PostHogUser): void {
  if (!client.__loaded) return;
  if (client.get_distinct_id() === user.id) return;

  client.identify(user.id, {
    name: user.name,
    email: user.email,
  });
}

export function resetPostHogIdentity(client: PostHogBrowserClient): void {
  if (!client.__loaded) return;

  client.reset();
}
