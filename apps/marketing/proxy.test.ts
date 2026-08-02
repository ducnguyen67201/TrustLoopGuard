import assert from 'node:assert/strict';
import test from 'node:test';
import { NextRequest } from 'next/server';
import { proxy } from './proxy';

function request(headers: HeadersInit = {}) {
  return new NextRequest('https://featherlane.ai/?source=test', { headers });
}

test('redirects a visitor from Vietnam using the Vercel country header', () => {
  const response = proxy(request({ 'x-vercel-ip-country': 'VN' }));

  assert.equal(response.status, 307);
  assert.equal(response.headers.get('location'), 'https://featherlane.ai/vi?source=test');
  assert.equal(response.headers.get('cache-control'), 'private, no-store');
});

test('redirects a visitor from Vietnam using the Cloudflare country header', () => {
  const response = proxy(request({ 'cf-ipcountry': 'vn' }));

  assert.equal(response.status, 307);
  assert.equal(response.headers.get('location'), 'https://featherlane.ai/vi?source=test');
});

test('redirects a visitor from Vietnam using the CloudFront country header', () => {
  const response = proxy(request({ 'cloudfront-viewer-country': 'VN' }));

  assert.equal(response.status, 307);
  assert.equal(response.headers.get('location'), 'https://featherlane.ai/vi?source=test');
});

test('keeps visitors outside Vietnam on the English homepage', () => {
  const response = proxy(request({ 'accept-language': 'vi-VN,vi;q=0.9', 'cf-ipcountry': 'FR' }));

  assert.equal(response.status, 200);
  assert.equal(response.headers.get('x-middleware-next'), '1');
});

test('defaults to English when no geographic country header is available', () => {
  const response = proxy(request({ 'accept-language': 'vi-VN,vi;q=0.9' }));

  assert.equal(response.status, 200);
  assert.equal(response.headers.get('x-middleware-next'), '1');
});
