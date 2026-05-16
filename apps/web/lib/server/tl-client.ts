import 'server-only';
import { Client } from '@trustloopguard/sdk';
import { getServerUrl } from '../server-url';
import { getDashboardShell } from './dashboard-data';

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
  if (workspaceSlug === undefined || workspaceSlug === '') return tlClient();

  const shell = await getDashboardShell(workspaceSlug);
  return tlClient(shell.activeWorkspace.id);
}

function fetchWithWorkspace(workspaceId: string): typeof fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    headers.set('x-tlg-workspace-id', workspaceId);
    return globalThis.fetch(input, { ...init, headers });
  }) as typeof fetch;
}
