import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  handleProviderPayment,
} from './provider-adapter';
import { stripeTestKeyFromEnv } from './stripe';
import {
  DEFAULT_PROVIDER_PORT,
  type StripeRefundProviderRequest,
} from './types';

export {
  handleProviderPayment,
  isValidProviderAuthorization,
  providerApiKey,
  type ProviderReply,
} from './provider-adapter';

export function providerPort(): number {
  const raw = process.env.STRIPE_REFUND_PROVIDER_PORT?.trim();
  if (raw === undefined || raw === '') return DEFAULT_PROVIDER_PORT;
  const port = Number.parseInt(raw, 10);
  return Number.isInteger(port) && port > 0 && port <= 65_535 ? port : DEFAULT_PROVIDER_PORT;
}

export function providerBaseUrl(
  raw = process.env.STRIPE_REFUND_PROVIDER_BASE_URL,
): string {
  if (raw === undefined || raw.trim() === '') {
    return `http://127.0.0.1:${providerPort()}`;
  }

  const url = new URL(raw.trim());
  const isLoopback = ['127.0.0.1', 'localhost', '::1', '[::1]'].includes(url.hostname);
  if (url.protocol !== 'https:' && !(url.protocol === 'http:' && isLoopback)) {
    throw new Error('STRIPE_REFUND_PROVIDER_BASE_URL must use HTTPS or loopback HTTP');
  }
  if (url.username !== '' || url.password !== '' || url.search !== '' || url.hash !== '') {
    throw new Error('STRIPE_REFUND_PROVIDER_BASE_URL must not contain credentials, query, or fragment');
  }
  return url.toString().replace(/\/$/, '');
}

export function startProvider(): void {
  const server = createServer((req, res) => {
    void handleHttpRequest(req, res);
  });
  server.listen(providerPort(), '127.0.0.1', () => {
    process.stdout.write(`Stripe refund provider listening on ${providerBaseUrl()}\n`);
    process.stdout.write('Stripe mode: ');
    process.stdout.write(stripeTestKeyFromEnv() === null ? 'simulated\n' : 'stripe-test\n');
  });
}

async function handleHttpRequest(req: IncomingMessage, res: ServerResponse): Promise<void> {
  if (req.method !== 'POST' || req.url !== '/payments') {
    writeJson(res, 404, { error: 'not found' });
    return;
  }

  try {
    const body = await readBody(req);
    const request = JSON.parse(body) as StripeRefundProviderRequest;
    const reply = await handleProviderPayment(req.headers.authorization, request);
    writeJson(res, reply.statusCode, reply.body);
  } catch (error) {
    writeJson(res, 500, {
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolveBody, reject) => {
    let body = '';
    req.setEncoding('utf8');
    req.on('data', (chunk) => {
      body += chunk;
    });
    req.on('end', () => resolveBody(body));
    req.on('error', reject);
  });
}

function writeJson(res: ServerResponse, statusCode: number, body: object): void {
  res.writeHead(statusCode, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
}

function isMainModule(): boolean {
  return resolve(process.argv[1] ?? '') === fileURLToPath(import.meta.url);
}

if (isMainModule()) {
  startProvider();
}
