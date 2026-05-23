import 'server-only';
import { Client } from '@trustloopguard/sdk';
import { auth } from '@/auth';
import { getServerUrl } from '../server-url';
import { selectAuthorizedWorkspaceId, type WorkspaceMembership } from '../workspace-access';
import { env } from '@/env';

const DEFAULT_WORKSPACE_SLUG = 'trustloop-demo';
const DEFAULT_WORKSPACE_ID = 'ws_trustloop_demo';

let cached: Client | null = null;

export class RustApiError extends Error {
  constructor(
    public readonly path: string,
    public readonly status: number,
    public readonly body: string,
  ) {
    super(`Rust API ${path} failed with ${status}: ${body}`);
  }
}

export class WorkspaceAccessError extends Error {
  constructor(
    public readonly status: 401 | 403,
    message: string,
  ) {
    super(message);
  }
}

export function tlClient(workspaceId?: string): Client {
  if (workspaceId !== undefined && workspaceId.trim() !== '') {
    return new Client({
      baseUrl: getServerUrl(),
      fetchImpl: fetchWithWorkspace(workspaceId.trim()),
    });
  }
  if (cached !== null) return cached;
  cached = new Client({
    baseUrl: getServerUrl(),
    fetchImpl: globalThis.fetch.bind(globalThis),
  });
  return cached;
}

export async function tlClientForRequest(req: Request): Promise<Client> {
  return tlClient(await authorizedWorkspaceIdForRequest(req));
}

export async function rustApiForAuthorizedWorkspace<T>(
  req: Request,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  return rustApiForWorkspace<T>(await authorizedWorkspaceIdForRequest(req), path, init);
}

export async function authorizedWorkspaceIdForRequest(req: Request): Promise<string> {
  const user = await userFromSession();
  if (user === null) {
    throw new WorkspaceAccessError(401, 'authentication required');
  }

  const requested = new URL(req.url).searchParams.get('workspace');
  const data = await rustApiForUser<{ workspaces: WorkspaceMembership[] }>(
    user,
    '/v1/team/my-workspaces',
  );
  const workspaceId = selectAuthorizedWorkspaceId(data.workspaces, requested);
  if (workspaceId === null) {
    throw new WorkspaceAccessError(403, 'workspace access denied');
  }
  return workspaceId;
}

export async function rustApiForWorkspace<T>(
  workspaceId: string,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set('x-tlg-workspace-id', workspaceId);
  applyInternalAuth(headers);
  const res = await fetch(`${getServerUrl()}${path}`, {
    ...init,
    headers,
  });
  if (res.status === 204) return undefined as T;
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new RustApiError(path, res.status, body);
  }
  return (await res.json()) as T;
}

/// Calls a Rust endpoint that scopes by user.
///
/// Auth precedence:
/// 1. If the caller has a Rust-issued JWT (credentials sign-in), we
///    send it as `Authorization: Bearer <jwt>`. The Rust middleware
///    verifies it and attaches a `UserContext` to the request, so
///    handlers can read `user_id` without trusting headers.
/// 2. Otherwise (OAuth users, who don't yet get a Rust JWT), we
///    fall back to `X-TLG-User-Id` + `X-TLG-User-Email` forwarding.
///    The trusted internal `TL_API_KEY` lane authorizes the request.
///
/// Both lanes work side by side; the JWT path is preferred whenever
/// it's available.
export async function rustApiForUser<T>(
  user: {
    id: string;
    email?: string | null | undefined;
    tlJwt?: string | null | undefined;
  },
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  const jwt = user.tlJwt?.trim();
  if (jwt !== undefined && jwt !== '') {
    headers.set('authorization', `Bearer ${jwt}`);
  } else {
    applyInternalAuth(headers);
  }
  // Always forward identity headers too — useful for the auto-bind
  // path (email lookup) and as a fallback when no JWT is present.
  headers.set('x-tlg-user-id', user.id);
  if (user.email !== undefined && user.email !== null && user.email.trim() !== '') {
    headers.set('x-tlg-user-email', user.email.trim());
  }
  const res = await fetch(`${getServerUrl()}${path}`, {
    ...init,
    headers,
  });
  if (res.status === 204) return undefined as T;
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new RustApiError(path, res.status, body);
  }
  return (await res.json()) as T;
}

function fetchWithWorkspace(workspaceId: string): typeof fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    headers.set('x-tlg-workspace-id', workspaceId);
    applyInternalAuth(headers);
    return globalThis.fetch(input, { ...init, headers });
  }) as typeof fetch;
}

function applyInternalAuth(headers: Headers) {
  const key = env.TL_API_KEY?.trim();
  if (key !== undefined && key !== '') {
    headers.set('authorization', `Bearer ${key}`);
  }
}

export function workspaceIdFromSlug(workspaceSlug?: string | null): string {
  const slug = normalizeWorkspaceSlug(workspaceSlug);
  if (slug === DEFAULT_WORKSPACE_SLUG || slug === 'default') return DEFAULT_WORKSPACE_ID;
  if (slug.startsWith('ws_')) return slug;
  return `ws_${slug.replace(/-/g, '_')}`;
}

export function normalizeWorkspaceSlug(workspaceSlug?: string | null): string {
  const slug = workspaceSlug?.trim();
  return slug && slug.length > 0 ? slug : DEFAULT_WORKSPACE_SLUG;
}

async function userFromSession(): Promise<{
  id: string;
  email?: string | null;
  tlJwt?: string | null;
} | null> {
  const session = await auth();
  const user = session?.user as
    | { id?: string; email?: string | null; tlJwt?: string | null }
    | undefined;
  if (user?.id === undefined || user.id.trim() === '') {
    return null;
  }

  return {
    id: user.id,
    ...(user.email !== undefined ? { email: user.email } : {}),
    ...(user.tlJwt !== undefined ? { tlJwt: user.tlJwt } : {}),
  };
}
