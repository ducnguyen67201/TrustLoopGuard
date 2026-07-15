import type {
  CreateGatewayProviderConnectionRequest,
  CreateFinancialPolicyRequest,
  GatewayProviderConnection,
  GatewayProviderConnectionListResponse,
  UpdateGatewayProviderConnectionRequest,
} from '@trustloopguard/sdk';

import { ADMIN_USER_ID, API_KEY, createClient, SERVER_URL, WORKSPACE_ID } from '../shared/env';
import { orderDatabasePath, resetOrderDatabase } from './order-db';
import { providerApiKey, providerBaseUrl } from './provider';

async function main(): Promise<void> {
  resetOrderDatabase();
  process.stdout.write(`SQLite order DB ready: ${orderDatabasePath()}\n`);

  const policy = await ensureFinancialControl();
  process.stdout.write(`financial control ready: ${policy.id}\n`);

  const connection = await ensureProviderConnection();
  process.stdout.write(`payment_http provider ready: ${connection.id}\n\n`);

  process.stdout.write('Next terminals:\n');
  process.stdout.write('  pnpm --filter @trustloopguard/demo stripe-refund-agent:provider\n');
  process.stdout.write('  pnpm --filter @trustloopguard/demo stripe-refund-agent\n');
}

async function ensureFinancialControl(): Promise<{ id: string }> {
  const client = createClient();
  return client.createFinancialPolicy(REFUND_CONTROL);
}

const REFUND_CONTROL: CreateFinancialPolicyRequest = {
  id: 'refund-bot-refund-controls',
  description: 'Refund controls for support agents',
  severity: 'high',
  meter: 'actions',
  when: {
    agents: ['refund-bot'],
    action_kinds: ['refund'],
    operations: ['issue_refund'],
    currencies: ['USD'],
    rails: ['payment_http'],
  },
  per_transaction_minor: 10_000n,
  approval_threshold_minor: 5_000n,
  allowed_counterparty_ids: [],
  denied_counterparty_ids: [],
  require_approval_for_new_counterparty: false,
  grant_required: false,
  approver_roles: [],
  refund_original_method_only: false,
  required_preconditions: [
    'order_exists',
    'payment_captured',
    'refund_window_open',
    'amount_lte_refundable_balance',
    'destination_is_original_payment_method',
    'no_duplicate_refund',
  ],
  missing_evidence_effect: 'defer',
  failed_precondition_effect: 'deny',
  on_breach: 'deny',
};

async function ensureProviderConnection(): Promise<GatewayProviderConnection> {
  const existing = await listProviderConnections();
  const targetBaseUrl = providerBaseUrl();
  const match = existing.provider_connections.find(
    (connection) => connection.kind === 'payment_http' && connection.base_url === targetBaseUrl,
  );
  if (match !== undefined) return match;

  const currentPaymentConnection = existing.provider_connections.find(
    (connection) => connection.kind === 'payment_http',
  );
  if (currentPaymentConnection !== undefined) {
    return updateProviderConnection(currentPaymentConnection.id, targetBaseUrl);
  }

  const body: CreateGatewayProviderConnectionRequest = {
    display_name: 'Stripe refund sandbox',
    kind: 'payment_http',
    base_url: targetBaseUrl,
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

async function updateProviderConnection(
  connectionId: string,
  baseUrl: string,
): Promise<GatewayProviderConnection> {
  const body: UpdateGatewayProviderConnectionRequest = {
    display_name: 'Stripe refund sandbox',
    base_url: baseUrl,
    default_model: 'payment-http',
    provider_api_key: providerApiKey(),
  };
  const res = await fetch(
    `${SERVER_URL}/v1/gateway/provider-connections/${encodeURIComponent(connectionId)}`,
    {
      method: 'PATCH',
      headers: jsonHeaders(),
      body: JSON.stringify(body),
    },
  );
  const text = await res.text();
  if (!res.ok) {
    throw new Error(`provider connection update failed: ${res.status} ${text}`);
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
  if (ADMIN_USER_ID) headers['x-tlg-user-id'] = ADMIN_USER_ID;
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
