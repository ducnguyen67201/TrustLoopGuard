import {
  Client,
  type ApiKeyListResponse,
  type CreateApiKeyResponse,
  type DashboardApiKey,
} from '@trustloopguard/sdk';

import {
  ADMIN_USER_ID,
  API_KEY,
  CONTEXTUAL_DEMO_API_KEY,
  SERVER_URL,
} from '../shared/env';
import {
  CONTEXTUAL_DEMO_AGENT_ID,
  CONTEXTUAL_DEMO_WORKSPACE_NAME,
  CONTEXTUAL_DISABLED_STARTER_POLICY_IDS,
} from './config';

export const CONTEXTUAL_RUNTIME_KEY_NAME = 'Contextual demo runtime';

export interface ContextualWorkspace {
  id: string;
  slug: string;
  name: string;
}

interface ContextualWorkspaceListResponse {
  workspaces: ContextualWorkspace[];
}

export interface ContextualEnvironment {
  id: string;
  slug: string;
  name: string;
  is_default: boolean;
}

interface ContextualEnvironmentListResponse {
  environments: ContextualEnvironment[];
}

export interface ContextualWorkspaceAdminConfig {
  serverUrl: string;
  internalApiKey?: string;
  adminUserId: string;
  fetchImpl: typeof fetch;
  runtimeApiKey?: string;
}

export type ContextualRuntimeKeyResult =
  | { status: 'created'; apiKey: DashboardApiKey; plaintextKey: string }
  | { status: 'existing'; apiKey: DashboardApiKey };

interface ContextualPolicyProvisioner {
  listPolicies: Client['listPolicies'];
  batchSetPolicyEnabled: Client['batchSetPolicyEnabled'];
}

export function contextualWorkspaceAdminConfigFromEnv(): ContextualWorkspaceAdminConfig {
  if (ADMIN_USER_ID === undefined) {
    throw new Error('TL_ADMIN_USER_ID is required to manage the Contextual Demo workspace');
  }
  return {
    serverUrl: SERVER_URL,
    ...(API_KEY === undefined ? {} : { internalApiKey: API_KEY }),
    adminUserId: ADMIN_USER_ID,
    fetchImpl: globalThis.fetch.bind(globalThis),
    ...(CONTEXTUAL_DEMO_API_KEY === undefined
      ? {}
      : { runtimeApiKey: CONTEXTUAL_DEMO_API_KEY }),
  };
}

export async function ensureContextualWorkspace(
  config: ContextualWorkspaceAdminConfig,
): Promise<ContextualWorkspace> {
  const response = await adminRequest<ContextualWorkspaceListResponse>(
    config,
    '/v1/team/my-workspaces',
  );
  const matches = response.workspaces.filter(
    (workspace) => workspace.name === CONTEXTUAL_DEMO_WORKSPACE_NAME,
  );
  if (matches.length > 1) {
    throw new Error(
      `multiple workspaces named "${CONTEXTUAL_DEMO_WORKSPACE_NAME}" exist; rename duplicates before rerunning setup`,
    );
  }
  if (matches[0] !== undefined) return matches[0];

  return adminRequest<ContextualWorkspace>(config, '/v1/team/my-workspaces', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ name: CONTEXTUAL_DEMO_WORKSPACE_NAME }),
  });
}

export async function resolveContextualEnvironment(
  workspaceId: string,
  config: ContextualWorkspaceAdminConfig,
): Promise<ContextualEnvironment> {
  const response = await adminRequest<ContextualEnvironmentListResponse>(
    config,
    '/v1/environments',
    {},
    workspaceId,
  );
  const defaults = response.environments.filter((environment) => environment.is_default);
  if (defaults.length !== 1 || defaults[0] === undefined) {
    throw new Error(
      `Contextual Demo workspace must have exactly one default environment; found ${defaults.length}`,
    );
  }
  return defaults[0];
}

export function createContextualManagementClient(
  workspaceId: string,
  environmentId: string,
  config: ContextualWorkspaceAdminConfig,
): Client {
  return new Client({
    baseUrl: config.serverUrl,
    apiKey: config.internalApiKey,
    fetchImpl: scopedAdminFetch(config, workspaceId, environmentId),
  });
}

export async function disableContextualStarterPolicies(
  client: ContextualPolicyProvisioner,
): Promise<string[]> {
  const starterIds = new Set<string>(CONTEXTUAL_DISABLED_STARTER_POLICY_IDS);
  const inventory = await client.listPolicies({ family: 'content' });
  const enabledStarterIds = inventory.policies
    .filter((policy) => policy.enabled && starterIds.has(policy.id))
    .map((policy) => policy.id);
  if (enabledStarterIds.length > 0) {
    await client.batchSetPolicyEnabled(enabledStarterIds, false);
  }
  return enabledStarterIds;
}

export async function ensureContextualRuntimeKey(
  workspaceId: string,
  environmentId: string,
  config: ContextualWorkspaceAdminConfig,
): Promise<ContextualRuntimeKeyResult> {
  const response = await adminRequest<ApiKeyListResponse>(config, '/v1/api-keys', {}, workspaceId);
  const namedActiveKeys = response.api_keys.filter(
    (apiKey) =>
      apiKey.name === CONTEXTUAL_RUNTIME_KEY_NAME &&
      apiKey.status === 'active' &&
      apiKey.environment_id === environmentId,
  );
  const incompatible = namedActiveKeys.find(
    (apiKey) => apiKey.principal_id !== CONTEXTUAL_DEMO_AGENT_ID,
  );
  if (incompatible !== undefined) {
    throw new Error(
      `${CONTEXTUAL_RUNTIME_KEY_NAME} key ${incompatible.id} is not bound to ${CONTEXTUAL_DEMO_AGENT_ID}`,
    );
  }
  if (namedActiveKeys.length > 1) {
    throw new Error(`multiple active ${CONTEXTUAL_RUNTIME_KEY_NAME} keys exist; revoke duplicates`);
  }
  const existing = namedActiveKeys[0];
  if (existing !== undefined) {
    verifyConfiguredRuntimeKey(existing, config.runtimeApiKey);
    return { status: 'existing', apiKey: existing };
  }

  const created = await adminRequest<CreateApiKeyResponse>(
    config,
    '/v1/api-keys',
    {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        name: CONTEXTUAL_RUNTIME_KEY_NAME,
        environment_id: environmentId,
        principal_id: CONTEXTUAL_DEMO_AGENT_ID,
      }),
    },
    workspaceId,
  );
  return {
    status: 'created',
    apiKey: created.api_key,
    plaintextKey: created.plaintext_key,
  };
}

function verifyConfiguredRuntimeKey(
  existing: DashboardApiKey,
  configuredRuntimeKey: string | undefined,
): void {
  const configured = configuredRuntimeKey?.trim();
  if (configured === undefined || configured === '') return;
  if (!configured.startsWith(existing.prefix)) {
    throw new Error('TL_CONTEXTUAL_DEMO_API_KEY does not match the active runtime key');
  }
}

async function adminRequest<ResponseBody>(
  config: ContextualWorkspaceAdminConfig,
  path: string,
  init: RequestInit = {},
  workspaceId?: string,
): Promise<ResponseBody> {
  const response = await config.fetchImpl(`${config.serverUrl.replace(/\/$/, '')}${path}`, {
    ...init,
    headers: adminHeaders(config, init.headers, workspaceId),
  });
  if (!response.ok) {
    const body = await response.text().catch(() => '');
    throw new Error(`Contextual workspace request ${path} failed with ${response.status}: ${body}`);
  }
  return (await response.json()) as ResponseBody;
}

function scopedAdminFetch(
  config: ContextualWorkspaceAdminConfig,
  workspaceId: string,
  environmentId: string,
): typeof fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) =>
    config.fetchImpl(input, {
      ...init,
      headers: adminHeaders(config, init?.headers, workspaceId, environmentId),
    })) as typeof fetch;
}

function adminHeaders(
  config: ContextualWorkspaceAdminConfig,
  initial: HeadersInit | undefined,
  workspaceId?: string,
  environmentId?: string,
): Headers {
  const headers = new Headers(initial);
  if (config.internalApiKey !== undefined && config.internalApiKey.trim() !== '') {
    headers.set('authorization', `Bearer ${config.internalApiKey}`);
  }
  headers.set('x-tlg-user-id', config.adminUserId);
  if (workspaceId !== undefined) headers.set('x-tlg-workspace-id', workspaceId);
  if (environmentId !== undefined) headers.set('x-tlg-environment-id', environmentId);
  return headers;
}
