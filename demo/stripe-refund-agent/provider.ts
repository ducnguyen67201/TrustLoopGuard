import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createStripeRefund, stripeTestKeyFromEnv, type StripeFetch } from './stripe';
import {
  DEFAULT_PROVIDER_API_KEY,
  DEFAULT_PROVIDER_PORT,
  type StripeRefundProviderRequest,
  type StripeRefundProviderResponse,
} from './types';

export interface ProviderReply {
  statusCode: number;
  body: StripeRefundProviderResponse | { error: string };
}

export async function handleProviderPayment(
  authorization: string | undefined,
  request: StripeRefundProviderRequest,
  fetchImpl?: StripeFetch,
): Promise<ProviderReply> {
  const expected = providerApiKey();
  if (authorization !== `Bearer ${expected}`) {
    return { statusCode: 401, body: { error: 'invalid provider bearer token' } };
  }
  if (request.kind !== 'refund') {
    return { statusCode: 400, body: { error: 'provider only supports refund actions' } };
  }
  if (request.currency !== 'USD') {
    return { statusCode: 400, body: { error: 'provider only supports USD refunds' } };
  }

  const amountMinor = request.amount_minor ?? request.amount;
  if (!Number.isInteger(amountMinor) || amountMinor === undefined || amountMinor <= 0) {
    return { statusCode: 400, body: { error: 'amount_minor must be a positive integer' } };
  }

  const paymentIntentId = request.metadata?.payment_intent_id;
  if (paymentIntentId === undefined || paymentIntentId.trim() === '') {
    return { statusCode: 400, body: { error: 'metadata.payment_intent_id is required' } };
  }

  const stripeKey = stripeTestKeyFromEnv();
  if (stripeKey === null) {
    return {
      statusCode: 200,
      body: providerSuccess({
        providerReference: `simulated_re_${request.action_id}`,
        providerStatus: 'succeeded',
        mode: 'simulated',
      }),
    };
  }

  const refund = await createStripeRefund({
    secretKey: stripeKey,
    paymentIntentId,
    amountMinor,
    reason: request.metadata?.reason ?? 'customer_request',
    idempotencyKey: request.action_id,
    fetchImpl,
  });
  return {
    statusCode: 200,
    body: providerSuccess({
      providerReference: refund.id,
      providerStatus: refund.status,
      mode: 'stripe-test',
      stripeRefundId: refund.id,
    }),
  };
}

export function providerApiKey(
  raw = process.env.STRIPE_REFUND_PROVIDER_API_KEY,
  nodeEnv = process.env.NODE_ENV,
): string {
  const key = raw?.trim() || DEFAULT_PROVIDER_API_KEY;
  if (nodeEnv === 'production' && key.length < 32) {
    throw new Error('STRIPE_REFUND_PROVIDER_API_KEY must contain at least 32 characters');
  }
  return key;
}

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
  if (
    url.username !== '' ||
    url.password !== '' ||
    url.search !== '' ||
    url.hash !== '' ||
    (url.pathname !== '' && url.pathname !== '/')
  ) {
    throw new Error('STRIPE_REFUND_PROVIDER_BASE_URL must be a plain service origin');
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

function providerSuccess(input: {
  providerReference: string;
  providerStatus: string;
  mode: 'simulated' | 'stripe-test';
  stripeRefundId?: string;
}): StripeRefundProviderResponse {
  return {
    status: 'succeeded',
    provider_status: input.providerStatus,
    provider_reference: input.providerReference,
    reversal_capability: 'manual_recovery',
    recovery_status: 'manual_required',
    mode: input.mode,
    stripe_refund_id: input.stripeRefundId,
  };
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
