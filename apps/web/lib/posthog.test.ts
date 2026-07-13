import { describe, expect, it, vi } from 'vitest';

import {
  identifyPostHogUser,
  initializeDashboardPostHog,
  resetPostHogIdentity,
  type PostHogBrowserClient,
} from './posthog';

function fakeClient(distinctId = 'anonymous-id', loaded = true) {
  return {
    __loaded: loaded,
    init: vi.fn<PostHogBrowserClient['init']>(),
    register: vi.fn<PostHogBrowserClient['register']>(),
    identify: vi.fn<PostHogBrowserClient['identify']>(),
    get_distinct_id: vi.fn<PostHogBrowserClient['get_distinct_id']>(() => distinctId),
    reset: vi.fn<PostHogBrowserClient['reset']>(),
  } satisfies PostHogBrowserClient;
}

describe('dashboard PostHog integration', () => {
  it('initializes analytics with the current PostHog defaults', () => {
    const client = fakeClient();

    const initialized = initializeDashboardPostHog(client, {
      projectToken: 'phc_test',
      host: 'https://us.i.posthog.com',
    });

    expect(initialized).toBe(true);
    expect(client.init).toHaveBeenCalledWith('phc_test', {
      api_host: 'https://us.i.posthog.com',
      defaults: '2026-05-30',
    });
    expect(client.register).toHaveBeenCalledWith({ app_surface: 'dashboard' });
  });

  it('keeps analytics disabled when no project token is configured', () => {
    const client = fakeClient();

    const initialized = initializeDashboardPostHog(client, {
      projectToken: undefined,
      host: 'https://us.i.posthog.com',
    });

    expect(initialized).toBe(false);
    expect(client.init).not.toHaveBeenCalled();
    expect(client.register).not.toHaveBeenCalled();
  });

  it('identifies an authenticated user with stable person properties', () => {
    const client = fakeClient();

    identifyPostHogUser(client, {
      id: 'user_123',
      name: 'Ada Lovelace',
      email: 'ada@example.com',
    });

    expect(client.identify).toHaveBeenCalledWith('user_123', {
      name: 'Ada Lovelace',
      email: 'ada@example.com',
    });
  });

  it('does not send another identify event for the current user', () => {
    const client = fakeClient('user_123');

    identifyPostHogUser(client, {
      id: 'user_123',
      name: 'Ada Lovelace',
      email: 'ada@example.com',
    });

    expect(client.identify).not.toHaveBeenCalled();
  });

  it('does not identify when PostHog is uninitialized', () => {
    const client = fakeClient('anonymous-id', false);

    identifyPostHogUser(client, {
      id: 'user_123',
      name: 'Ada Lovelace',
      email: 'ada@example.com',
    });

    expect(client.get_distinct_id).not.toHaveBeenCalled();
    expect(client.identify).not.toHaveBeenCalled();
  });

  it('resets browser identity during sign out', () => {
    const client = fakeClient('user_123');

    resetPostHogIdentity(client);

    expect(client.reset).toHaveBeenCalledOnce();
  });

  it('does not reset an uninitialized PostHog client', () => {
    const client = fakeClient('anonymous-id', false);

    resetPostHogIdentity(client);

    expect(client.reset).not.toHaveBeenCalled();
  });
});
