import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import type { AddressInfo } from 'node:net';
import { dirname, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { providerApiKey, providerBaseUrl } from './provider';
import {
  createRefundAgentServer,
  refundAgentHost,
  refundAgentPort,
} from './ui';

const PROVIDER_KEY = 'stripe-refund-provider-key-32-bytes-minimum';
const PROXY_SECRET = 'refund-demo-proxy-secret-32-bytes-minimum';

test('gives the non-root runtime a writable state directory', () => {
  const dockerfile = readFileSync(
    resolve(dirname(fileURLToPath(import.meta.url)), 'Dockerfile'),
    'utf8',
  );
  assert.match(dockerfile, /mkdir -p \/app\/demo\/\.data/);
  assert.match(dockerfile, /chown -R node:node \/app\/demo\/\.data/);
  assert.ok(dockerfile.indexOf('chown -R node:node') < dockerfile.indexOf('USER node'));
});

test('accepts Railway host and port configuration', () => {
  assert.equal(refundAgentHost('0.0.0.0'), '0.0.0.0');
  assert.equal(refundAgentPort('8080'), 8080);
  assert.equal(refundAgentPort('invalid'), 9310);
});

test('uses a validated remote provider origin for hosted setup', () => {
  assert.equal(
    providerBaseUrl('https://refund-demo-staging.up.railway.app/'),
    'https://refund-demo-staging.up.railway.app',
  );
  assert.throws(
    () => providerBaseUrl('http://refund-demo.example.com'),
    /HTTPS or loopback HTTP/,
  );
  assert.throws(
    () => providerBaseUrl('https://refund-demo.example.com?mode=unsafe'),
    /plain service origin/,
  );
});

test('requires a strong provider credential in production', () => {
  assert.throws(() => providerApiKey('short', 'production'), /at least 32 characters/);
  assert.equal(providerApiKey(PROVIDER_KEY, 'production'), PROVIDER_KEY);
});

test('serves the authenticated payment adapter on the refund service', async () => {
  const originalProviderKey = process.env['STRIPE_REFUND_PROVIDER_API_KEY'];
  const originalProxySecret = process.env['REFUND_DEMO_PROXY_SECRET'];
  const originalStripeKey = process.env['STRIPE_SECRET_KEY'];
  const originalNodeEnv = process.env['NODE_ENV'];
  process.env['STRIPE_REFUND_PROVIDER_API_KEY'] = PROVIDER_KEY;
  process.env['REFUND_DEMO_PROXY_SECRET'] = PROXY_SECRET;
  delete process.env['STRIPE_SECRET_KEY'];
  process.env['NODE_ENV'] = 'production';

  const server = createRefundAgentServer();
  try {
    await new Promise<void>((resolve, reject) => {
      server.once('error', reject);
      server.listen(0, '127.0.0.1', resolve);
    });
    const address = server.address() as AddressInfo;
    const serviceUrl = `http://127.0.0.1:${address.port}`;
    const health = await fetch(`${serviceUrl}/health`);
    assert.equal(health.status, 200);
    assert.deepEqual(await health.json(), { status: 'ok' });

    const url = `${serviceUrl}/payments`;
    const request = {
      action_id: 'financial_action_deploy_test',
      kind: 'refund',
      amount_minor: 2_500,
      currency: 'USD',
      metadata: {
        payment_intent_id: 'pi_demo_deploy_test',
        reason: 'damaged_item',
      },
    };

    const rejected = await fetch(url, {
      method: 'POST',
      headers: {
        authorization: 'Bearer wrong',
        'content-type': 'application/json',
      },
      body: JSON.stringify(request),
    });
    assert.equal(rejected.status, 401);

    const accepted = await fetch(url, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${PROVIDER_KEY}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify(request),
    });
    assert.equal(accepted.status, 200);
    assert.deepEqual(await accepted.json(), {
      status: 'succeeded',
      provider_status: 'succeeded',
      provider_reference: 'simulated_re_financial_action_deploy_test',
      reversal_capability: 'manual_recovery',
      recovery_status: 'manual_required',
      mode: 'simulated',
    });
  } finally {
    await new Promise<void>((resolve) => server.close(() => resolve()));
    restoreEnv('STRIPE_REFUND_PROVIDER_API_KEY', originalProviderKey);
    restoreEnv('REFUND_DEMO_PROXY_SECRET', originalProxySecret);
    restoreEnv('STRIPE_SECRET_KEY', originalStripeKey);
    restoreEnv('NODE_ENV', originalNodeEnv);
  }
});

function restoreEnv(name: string, value: string | undefined): void {
  if (value === undefined) delete process.env[name];
  else process.env[name] = value;
}
