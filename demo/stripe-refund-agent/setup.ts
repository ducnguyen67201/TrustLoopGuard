import type {
  CreateGatewayProviderConnectionRequest,
  GatewayProviderConnection,
  GatewayProviderConnectionListResponse,
} from '@trustloopguard/sdk';

import { createClient, API_KEY, SERVER_URL, WORKSPACE_ID } from '../shared/env';
import { ensureRefundMandate } from './core';
import { providerApiKey, providerBaseUrl } from './provider';

async function main(): Promise<void> {
  const client = createClient();
  const mandate = await ensureRefundMandate(client);
  process.stdout.write(`refund mandate ready: ${mandate.id} v${mandate.version}\n`);

  const connection = await ensureProviderConnection();
  process.stdout.write(`payment_http provider ready: ${connection.id}\n\n`);

  process.stdout.write('Next terminals:\n');
  process.stdout.write('  pnpm --filter @trustloopguard/demo stripe-refund-agent:provider\n');
  process.stdout.write('  pnpm --filter @trustloopguard/demo stripe-refund-agent\n');
}

async function ensureProviderConnection(): Promise<GatewayProviderConnection> {
  const existing = await listProviderConnections();
  const match = existing.provider_connections.find(
    (connection) => connection.kind === 'payment_http' && connection.base_url === providerBaseUrl(),
  );
  if (match !== undefined) return match;

  const body: CreateGatewayProviderConnectionRequest = {
    display_name: 'Stripe refund sandbox',
    kind: 'payment_http',
    base_url: providerBaseUrl(),
    default_model: 'payment-http',
    provider_api_key: providerApiKey(),
  };
  const res = await fetch(`${SERVER_URL}/v1/gateway/provider-connections`, {
    method: 'POST',
    headers: jsonHeaders(),
    body: JSON.stringify(body),
  });
  const text = await res.text();
  if (!res.ok) {
    throw new Error(`provider connection setup failed: ${res.status} ${text}`);
  }
  return JSON.parse(text) as GatewayProviderConnection;
}

async function listProviderConnections(): Promise<GatewayProviderConnectionListResponse> {
  const res = await fetch(`${SERVER_URL}/v1/gateway/provider-connections`, {
    method: 'GET',
    headers: jsonHeaders(),
  });
  const text = await res.text();
  if (!res.ok) {
    throw new Error(`provider connection list failed: ${res.status} ${text}`);
  }
  return JSON.parse(text) as GatewayProviderConnectionListResponse;
}

function jsonHeaders(): Record<string, string> {
  const headers: Record<string, string> = { 'content-type': 'application/json' };
  if (API_KEY) headers.authorization = `Bearer ${API_KEY}`;
  if (WORKSPACE_ID) headers['x-tlg-workspace-id'] = WORKSPACE_ID;
  return headers;
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  const hint = message.includes('missing bearer token')
    ? '\nSet TL_API_KEY for this local server, then rerun setup.'
    : '';
  process.stderr.write(
    `stripe refund agent setup failed: ${message}${hint}\n`,
  );
  process.exitCode = 1;
});
