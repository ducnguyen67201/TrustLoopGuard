import { beforeEach, describe, expect, it, vi } from 'vitest';

type JsonObject = { [key: string]: JsonValue };
type JsonValue = string | number | boolean | null | JsonObject | JsonValue[];

const mockTlClient = vi.hoisted(() => {
  class MockRustApiError extends Error {
    constructor(
      public readonly path: string,
      public readonly status: number,
      public readonly body: string,
    ) {
      super(`Rust API ${path} failed with ${status}: ${body}`);
    }
  }

  class MockWorkspaceAccessError extends Error {
    constructor(
      public readonly status: 401 | 403,
      message: string,
    ) {
      super(message);
    }
  }

  return {
    rustApiForAuthorizedWorkspace:
      vi.fn<(req: Request, path: string, init?: RequestInit) => Promise<JsonValue>>(),
    RustApiError: MockRustApiError,
    WorkspaceAccessError: MockWorkspaceAccessError,
  };
});

vi.mock('server-only', () => ({}));
vi.mock('@/lib/server/tl-client', () => ({
  RustApiError: mockTlClient.RustApiError,
  WorkspaceAccessError: mockTlClient.WorkspaceAccessError,
  rustApiForAuthorizedWorkspace: mockTlClient.rustApiForAuthorizedWorkspace,
}));

import { proxyRustCollection, proxyRustResource } from './proxy-helpers';

describe('proxyRustCollection', () => {
  beforeEach(() => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockReset();
  });

  it('proxies GET collection requests through request-scoped workspace authorization', async () => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockResolvedValue({ routes: [] });
    const req = new Request('https://app.test/api/gateway/routes?workspace=acme');

    const res = await proxyRustCollection(req, '/v1/gateway/routes', 'GET');

    expect(mockTlClient.rustApiForAuthorizedWorkspace).toHaveBeenCalledWith(
      req,
      '/v1/gateway/routes',
      { method: 'GET' },
    );
    expect(res.status).toBe(200);
    await expect(res.json()).resolves.toEqual({ routes: [] });
  });

  it('proxies POST collection requests with a JSON body and created status', async () => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockResolvedValue({ id: 'route_1' });
    const req = new Request('https://app.test/api/gateway/routes?workspace=acme', {
      method: 'POST',
      body: JSON.stringify({ name: 'Primary' }),
    });

    const res = await proxyRustCollection(req, '/v1/gateway/routes', 'POST');

    expect(mockTlClient.rustApiForAuthorizedWorkspace).toHaveBeenCalledWith(
      req,
      '/v1/gateway/routes',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: 'Primary' }),
      },
    );
    expect(res.status).toBe(201);
    await expect(res.json()).resolves.toEqual({ id: 'route_1' });
  });

  it('returns workspace authorization errors from the request guard', async () => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockRejectedValue(
      new mockTlClient.WorkspaceAccessError(403, 'workspace access denied'),
    );
    const req = new Request('https://app.test/api/gateway/routes?workspace=other-team');

    const res = await proxyRustCollection(req, '/v1/gateway/routes', 'GET');

    expect(res.status).toBe(403);
    await expect(res.json()).resolves.toEqual({ error: 'workspace access denied' });
  });

  it('maps Rust API errors without hiding client-side statuses', async () => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockRejectedValue(
      new mockTlClient.RustApiError('/v1/gateway/routes', 404, '{"error":"not found"}'),
    );
    const req = new Request('https://app.test/api/gateway/routes?workspace=acme');

    const res = await proxyRustCollection(req, '/v1/gateway/routes', 'GET');

    expect(res.status).toBe(404);
    await expect(res.json()).resolves.toEqual({ error: 'not found' });
  });
});

describe('proxyRustResource', () => {
  beforeEach(() => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockReset();
  });

  it('proxies PATCH resource requests through request-scoped workspace authorization', async () => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockResolvedValue({ id: 'route/a b' });
    const req = new Request('https://app.test/api/gateway/routes/route%2Fa%20b?workspace=acme', {
      method: 'PATCH',
      body: JSON.stringify({ enabled: false }),
    });

    const res = await proxyRustResource(
      req,
      Promise.resolve({ id: 'route/a b' }),
      '/v1/gateway/routes',
      'PATCH',
    );

    expect(mockTlClient.rustApiForAuthorizedWorkspace).toHaveBeenCalledWith(
      req,
      '/v1/gateway/routes/route%2Fa%20b',
      {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: false }),
      },
    );
    expect(res.status).toBe(200);
    await expect(res.json()).resolves.toEqual({ id: 'route/a b' });
  });

  it('proxies PUT resource requests with a suffix and JSON body', async () => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockResolvedValue({ user_ids: ['user-1'] });
    const body = JSON.stringify({ user_ids: ['user-1'] });
    const req = new Request('https://app.test/api/mcp-gateway/tools/tool-1/assignments', {
      method: 'PUT',
      body,
    });

    const res = await proxyRustResource(
      req,
      Promise.resolve({ id: 'tool-1' }),
      '/v1/mcp-gateway/tools',
      'PUT',
      'assignments',
    );

    expect(mockTlClient.rustApiForAuthorizedWorkspace).toHaveBeenCalledWith(
      req,
      '/v1/mcp-gateway/tools/tool-1/assignments',
      { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body },
    );
    expect(res.status).toBe(200);
  });

  it('proxies POST resource actions without inventing a request body', async () => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockResolvedValue({ tool_count: 3 });
    const req = new Request('https://app.test/api/mcp-gateway/connections/connection-1/sync', {
      method: 'POST',
    });

    const res = await proxyRustResource(
      req,
      Promise.resolve({ id: 'connection-1' }),
      '/v1/mcp-gateway/connections',
      'POST',
      'sync',
    );

    expect(mockTlClient.rustApiForAuthorizedWorkspace).toHaveBeenCalledWith(
      req,
      '/v1/mcp-gateway/connections/connection-1/sync',
      { method: 'POST' },
    );
    expect(res.status).toBe(200);
  });
});

describe('proxyRustResource DELETE', () => {
  beforeEach(() => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockReset();
  });

  it('proxies DELETE resource requests and returns no content', async () => {
    mockTlClient.rustApiForAuthorizedWorkspace.mockResolvedValue(null);
    const req = new Request(
      'https://app.test/api/gateway/provider-connections/provider%2Fa?workspace=acme',
      { method: 'DELETE' },
    );

    const res = await proxyRustResource(
      req,
      Promise.resolve({ id: 'provider/a' }),
      '/v1/gateway/provider-connections',
      'DELETE',
    );

    expect(mockTlClient.rustApiForAuthorizedWorkspace).toHaveBeenCalledWith(
      req,
      '/v1/gateway/provider-connections/provider%2Fa',
      { method: 'DELETE' },
    );
    expect(res.status).toBe(204);
    await expect(res.text()).resolves.toBe('');
  });
});
