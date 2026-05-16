import 'server-only';
import { Client } from '@trustloopguard/sdk';
import { getServerUrl } from '../server-url';

const DEFAULT_WORKSPACE_SLUG = 'trustloop-demo';
const DEFAULT_WORKSPACE_ID = 'ws_trustloop_demo';

let cached: Client | null = null;

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
  const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
  return tlClient(workspaceIdFromSlug(workspaceSlug));
}

export async function rustApiForWorkspace<T>(
  workspaceId: string,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set('x-tlg-workspace-id', workspaceId);
  const res = await fetch(`${getServerUrl()}${path}`, {
    ...init,
    headers,
  });
  if (res.status === 204) return undefined as T;
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new Error(`Rust API ${path} failed with ${res.status}: ${body}`);
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
    throw new Error(`Rust API ${path} failed with ${res.status}: ${body}`);
  }
  return (await res.json()) as T;
}

function fetchWithWorkspace(workspaceId: string): typeof fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    headers.set('x-tlg-workspace-id', workspaceId);
    return globalThis.fetch(input, { ...init, headers });
  }) as typeof fetch;
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
