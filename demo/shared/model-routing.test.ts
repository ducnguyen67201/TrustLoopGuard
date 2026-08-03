import assert from 'node:assert/strict';
import test from 'node:test';

import routingManifest from '../../config/llm-routing.json';
import { demoModelRoute, parseDemoModelRoute } from './model-routing';

test('loads committed demo routes from the canonical manifest', () => {
  assert.deepEqual(demoModelRoute('demo_default'), { model: 'gpt-4.1-mini' });
  assert.deepEqual(demoModelRoute('demo_dispute'), { model: 'gpt-4o-mini' });
});

test('returns configured reasoning effort', () => {
  const fixture = structuredClone(routingManifest);
  Object.assign(fixture.routes.demo_default.primary, { reasoning_effort: 'low' });

  assert.deepEqual(parseDemoModelRoute(fixture, 'demo_default'), {
    model: 'gpt-4.1-mini',
    reasoningEffort: 'low',
  });
});

test('rejects a missing demo route', () => {
  const fixture = structuredClone(routingManifest);
  Reflect.deleteProperty(fixture.routes, 'demo_dispute');

  assert.throws(
    () => parseDemoModelRoute(fixture, 'demo_dispute'),
    /missing route "demo_dispute"/,
  );
});

test('rejects invalid reasoning effort', () => {
  const fixture = structuredClone(routingManifest);
  Object.assign(fixture.routes.demo_default.primary, { reasoning_effort: 'fast' });

  assert.throws(() => parseDemoModelRoute(fixture, 'demo_default'), /Invalid LLM routing manifest/);
});

test('resolves an OpenAI provider alias through the provider map', () => {
  const fixture = structuredClone(routingManifest);
  Object.assign(fixture.providers, {
    first_party: { kind: 'openai' as const, api_key_env: 'OPENAI_API_KEY' },
  });
  fixture.routes.demo_default.primary.provider = 'first_party';

  assert.deepEqual(parseDemoModelRoute(fixture, 'demo_default'), {
    model: 'gpt-4.1-mini',
  });
});

test('rejects a provider identifier backed by a non-OpenAI provider', () => {
  const fixture = structuredClone(routingManifest);
  Object.assign(fixture.providers.openai, { kind: 'openrouter' as const });

  assert.throws(
    () => parseDemoModelRoute(fixture, 'demo_default'),
    /must use an openai provider/,
  );
});

test('rejects a route whose provider reference is missing', () => {
  const fixture = structuredClone(routingManifest);
  Reflect.deleteProperty(fixture.providers, 'openai');

  assert.throws(
    () => parseDemoModelRoute(fixture, 'demo_default'),
    /references missing provider "openai"/,
  );
});
