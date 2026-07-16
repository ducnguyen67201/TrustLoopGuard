import assert from 'node:assert/strict';
import test from 'node:test';

import {
  capturePostHogMarketingEvent,
  initializeMarketingPostHog,
  type PostHogBrowserClient,
} from './posthog';
import { trackMarketingEvent } from './gtm';

interface InitCall {
  token: string;
  options: {
    api_host: string;
    defaults: string;
  };
}

interface CaptureCall {
  event: string;
  properties: Record<string, string | undefined> | undefined;
}

function fakeClient(loaded = true) {
  const initCalls: InitCall[] = [];
  const registerCalls: Array<Record<string, string>> = [];
  const captureCalls: CaptureCall[] = [];

  const client: PostHogBrowserClient = {
    __loaded: loaded,
    init(token, options) {
      initCalls.push({ token, options });
    },
    register(properties) {
      registerCalls.push(properties);
    },
    capture(event, properties) {
      captureCalls.push({ event, properties });
    },
  };

  return { client, initCalls, registerCalls, captureCalls };
}

test('initializes marketing analytics with the current PostHog defaults', () => {
  const { client, initCalls, registerCalls } = fakeClient();

  const initialized = initializeMarketingPostHog(client, {
    projectToken: 'phc_test',
    host: 'https://us.i.posthog.com',
  });

  assert.equal(initialized, true);
  assert.deepEqual(initCalls, [
    {
      token: 'phc_test',
      options: {
        api_host: 'https://us.i.posthog.com',
        defaults: '2026-05-30',
      },
    },
  ]);
  assert.deepEqual(registerCalls, [{ app_surface: 'marketing' }]);
});

test('keeps marketing analytics disabled when no project token is configured', () => {
  const { client, initCalls, registerCalls } = fakeClient();

  const initialized = initializeMarketingPostHog(client, {
    projectToken: undefined,
    host: 'https://us.i.posthog.com',
  });

  assert.equal(initialized, false);
  assert.deepEqual(initCalls, []);
  assert.deepEqual(registerCalls, []);
});

test('captures the existing typed marketing event and its funnel properties', () => {
  const { client, captureCalls } = fakeClient();

  capturePostHogMarketingEvent(client, 'book_meeting_click', {
    page: '/use-cases',
    location: 'header',
    label: 'Book a demo',
  });

  assert.deepEqual(captureCalls, [
    {
      event: 'book_meeting_click',
      properties: {
        page: '/use-cases',
        location: 'header',
        label: 'Book a demo',
      },
    },
  ]);
});

test('captures the demo activation funnel with normalized properties', () => {
  const { client, captureCalls } = fakeClient();

  capturePostHogMarketingEvent(client, 'demo_decision_shown', {
    page: '/demo',
    location: 'refund_workflow',
    decision: 'require_approval',
    outcome: 'success',
  });

  assert.deepEqual(captureCalls, [
    {
      event: 'demo_decision_shown',
      properties: {
        page: '/demo',
        location: 'refund_workflow',
        decision: 'require_approval',
        outcome: 'success',
      },
    },
  ]);
});

test('sends one marketing interaction to both GTM and PostHog', () => {
  const { client, captureCalls } = fakeClient();
  const browser = { dataLayer: [] as Array<Record<string, string>> };
  Object.defineProperty(globalThis, 'window', { configurable: true, value: browser });

  trackMarketingEvent(
    'docs_click',
    { page: '/', location: 'footer', label: 'Documentation' },
    client,
  );

  assert.deepEqual(browser.dataLayer, [
    { event: 'docs_click', page: '/', location: 'footer', label: 'Documentation' },
  ]);
  assert.deepEqual(captureCalls, [
    {
      event: 'docs_click',
      properties: { page: '/', location: 'footer', label: 'Documentation' },
    },
  ]);
});

test('keeps GTM working without calling an uninitialized PostHog client', () => {
  const { client, captureCalls } = fakeClient(false);
  const browser = { dataLayer: [] as Array<Record<string, string>> };
  Object.defineProperty(globalThis, 'window', { configurable: true, value: browser });

  trackMarketingEvent('landing_cta_click', { page: '/', location: 'hero' }, client);

  assert.deepEqual(browser.dataLayer, [
    { event: 'landing_cta_click', page: '/', location: 'hero' },
  ]);
  assert.deepEqual(captureCalls, []);
});
