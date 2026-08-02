import {
  Client,
  type ApiKeyListResponse,
  type CreateApiKeyResponse,
  type DashboardApiKey,
} from '@featherlane-ai/sdk';

import {
  ADMIN_USER_ID,
  API_KEY,
  HEALTHCARE_DEMO_API_KEY,
  SERVER_URL,
} from '../shared/env';
import { HEALTHCARE_AGENT_ID, HEALTHCARE_WORKSPACE_NAME } from './config';

export { HEALTHCARE_WORKSPACE_NAME } from './config';

export const HEALTHCARE_RUNTIME_KEY_NAME = 'Healthcare demo runtime';

export interface HealthcareWorkspace {
  id: string;
  slug: string;
  name: string;
}

interface HealthcareWorkspaceListResponse {
  workspaces: HealthcareWorkspace[];
}

export interface HealthcareEnvironment {
  id: string;
  slug: string;
  name: string;
  is_default: boolean;
}

interface HealthcareEnvironmentListResponse {
  environments: HealthcareEnvironment[];
}

export interface HealthcareWorkspaceAdminConfig {
  serverUrl: string;
  internalApiKey?: string;
  adminUserId: string;
  fetchImpl: typeof fetch;
  runtimeApiKey?: string;
}

export type HealthcareRuntimeKeyResult =
  | { status: 'created'; apiKey: DashboardApiKey; plaintextKey: string }
  | { status: 'existing'; apiKey: DashboardApiKey };

export function healthcareWorkspaceAdminConfigFromEnv(): HealthcareWorkspaceAdminConfig {
  if (ADMIN_USER_ID === undefined) {
    throw new Error(
      'TL_ADMIN_USER_ID is required to create or manage the Healthcare Demo workspace',
    );
  }
  return {
    serverUrl: SERVER_URL,
    ...(API_KEY === undefined ? {} : { internalApiKey: API_KEY }),
    adminUserId: ADMIN_USER_ID,
    fetchImpl: globalThis.fetch.bind(globalThis),
    ...(HEALTHCARE_DEMO_API_KEY === undefined
      ? {}
      : { runtimeApiKey: HEALTHCARE_DEMO_API_KEY }),
  };
}

export async function ensureHealthcareWorkspace(
  config: HealthcareWorkspaceAdminConfig,
): Promise<HealthcareWorkspace> {
  const response = await adminRequest<HealthcareWorkspaceListResponse>(
    config,
    '/v1/team/my-workspaces',
  );
  const matches = response.workspaces.filter(
    (workspace) => workspace.name === HEALTHCARE_WORKSPACE_NAME,
  );
  if (matches.length > 1) {
    throw new Error(
      `multiple workspaces named "${HEALTHCARE_WORKSPACE_NAME}" exist; rename duplicates before rerunning setup`,
    );
  }
  if (matches[0] !== undefined) return matches[0];

  return adminRequest<HealthcareWorkspace>(config, '/v1/team/my-workspaces', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ name: HEALTHCARE_WORKSPACE_NAME }),
  });
}

export function createHealthcareManagementClient(
  workspaceId: string,
  environmentId: string,
  config: HealthcareWorkspaceAdminConfig,
): Client {
  return new Client({
    baseUrl: config.serverUrl,
    apiKey: config.internalApiKey,
    fetchImpl: scopedAdminFetch(config, workspaceId, environmentId),
  });
}

export async function resolveHealthcareEnvironment(
  workspaceId: string,
  config: HealthcareWorkspaceAdminConfig,
): Promise<HealthcareEnvironment> {
  const response = await adminRequest<HealthcareEnvironmentListResponse>(
    config,
    '/v1/environments',
    {},
    workspaceId,
  );
  const defaults = response.environments.filter((environment) => environment.is_default);
  if (defaults.length !== 1 || defaults[0] === undefined) {
    throw new Error(
      `Healthcare Demo workspace must have exactly one default environment; found ${defaults.length}`,
    );
  }
  return defaults[0];
}

export async function ensureHealthcareRuntimeKey(
  workspaceId: string,
  environmentId: string,
  config: HealthcareWorkspaceAdminConfig,
): Promise<HealthcareRuntimeKeyResult> {
  const response = await adminRequest<ApiKeyListResponse>(config, '/v1/api-keys', {}, workspaceId);
  const namedActiveKeys = response.api_keys.filter(
    (apiKey) =>
      apiKey.name === HEALTHCARE_RUNTIME_KEY_NAME &&
      apiKey.status === 'active' &&
      apiKey.environment_id === environmentId,
  );
  const incompatible = namedActiveKeys.find(
    (apiKey) => apiKey.principal_id !== HEALTHCARE_AGENT_ID,
  );
  if (incompatible !== undefined) {
    throw new Error(
      `${HEALTHCARE_RUNTIME_KEY_NAME} key ${incompatible.id} is not bound to ${HEALTHCARE_AGENT_ID}`,
    );
  }
  if (namedActiveKeys.length > 1) {
    throw new Error(
      `multiple active ${HEALTHCARE_RUNTIME_KEY_NAME} keys exist; revoke duplicates in the dashboard`,
    );
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
        name: HEALTHCARE_RUNTIME_KEY_NAME,
        environment_id: environmentId,
        principal_id: HEALTHCARE_AGENT_ID,
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
    throw new Error(
      'TL_HEALTHCARE_DEMO_API_KEY does not match the active Healthcare demo runtime key',
    );
  }
}

async function adminRequest<ResponseBody>(
  config: HealthcareWorkspaceAdminConfig,
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
    throw new Error(`Healthcare workspace request ${path} failed with ${response.status}: ${body}`);
  }
  return (await response.json()) as ResponseBody;
}

function scopedAdminFetch(
  config: HealthcareWorkspaceAdminConfig,
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
  config: HealthcareWorkspaceAdminConfig,
  initial: HeadersInit | undefined,
  workspaceId?: string,
  environmentId?: string,
): Headers {
  const headers = new Headers(initial);
  if (config.internalApiKey !== undefined && config.internalApiKey.trim() !== '') {
    headers.set('authorization', `Bearer ${config.internalApiKey}`);
  }
  headers.set('x-featherlane-ai-user-id', config.adminUserId);
  if (workspaceId !== undefined) headers.set('x-featherlane-ai-workspace-id', workspaceId);
  if (environmentId !== undefined) headers.set('x-featherlane-ai-environment-id', environmentId);
  return headers;
}
