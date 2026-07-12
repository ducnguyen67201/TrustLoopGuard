import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildFrustrationEvent,
  emitFrustrationEvent,
  reportSubscribeFrustration,
  submitWaitlist,
} from './subscribe-client.ts';

test('completes a successful signup request', async () => {
  let requestBody = '';
  const fetcher: typeof fetch = async (_input, init) => {
    requestBody = String(init?.body ?? '');
    return new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });
  };

  await submitWaitlist({ email: 'person@example.com' }, { fetcher });

  assert.equal(requestBody, JSON.stringify({ email: 'person@example.com' }));
});

test('times out a stalled signup request so the form can recover', async () => {
  const fetcher = (_input: string | URL | Request, init?: RequestInit) =>
    new Promise<Response>((_resolve, reject) => {
      init?.signal?.addEventListener('abort', () => reject(init.signal?.reason));
    });

  await assert.rejects(
    submitWaitlist({ email: 'person@example.com' }, { fetcher, timeoutMs: 5 }),
    /took too long/i,
  );
});

test('returns a stable error from a rejected signup response', async () => {
  const fetcher = async () =>
    new Response(JSON.stringify({ error: 'Waitlist is unavailable.' }), {
      status: 503,
      headers: { 'content-type': 'application/json' },
    });

  await assert.rejects(
    submitWaitlist({ email: 'person@example.com' }, { fetcher }),
    /Waitlist is unavailable/,
  );
});

test('builds clamped visual evidence without including the submitted email', () => {
  const event = buildFrustrationEvent({
    currentUrl: 'https://gettrustloop.app/',
    rect: { left: 1500, top: -20, width: 100, height: 40 },
    viewport: { width: 1200, height: 800 },
  });

  assert.equal(event.x, 100);
  assert.equal(event.y, 0);
  assert.equal(event.category, 'technical');
  assert.doesNotMatch(JSON.stringify(event), /person@example\.com/);
});

test('emits the exact GRANNY_EVENT prefix and JSON payload once', () => {
  const lines: string[] = [];
  const event = buildFrustrationEvent({
    currentUrl: 'https://gettrustloop.app/',
    rect: { left: 800, top: 600, width: 80, height: 40 },
    viewport: { width: 1000, height: 800 },
  });

  emitFrustrationEvent(event, (line) => lines.push(line));

  assert.equal(lines.length, 1);
  assert.match(lines[0] ?? '', /^GRANNY_EVENT \{"type":"report_frustration"/);
});

test('reports browser evidence around the failed submit control', () => {
  const lines: string[] = [];
  const originalInfo = console.info;
  const originalWindow = globalThis.window;
  console.info = (line) => lines.push(String(line));
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      innerHeight: 800,
      innerWidth: 1000,
      location: { href: 'https://gettrustloop.app/' },
    },
  });

  try {
    reportSubscribeFrustration({
      getBoundingClientRect: () => ({ left: 800, top: 600, width: 80, height: 40 }),
    } as HTMLElement);
  } finally {
    console.info = originalInfo;
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: originalWindow,
    });
  }

  assert.equal(lines.length, 1);
  assert.match(lines[0] ?? '', /"currentUrl":"https:\/\/gettrustloop\.app\/"/);
});
