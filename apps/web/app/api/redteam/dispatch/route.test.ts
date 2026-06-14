import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// `_shared` transitively imports the auth stack via tl-client; stub the module
// boundary like the other route tests so the proxy route loads in isolation.
vi.mock('@/lib/server/tl-client', () => {
  class MockRustApiError extends Error {}
  class MockWorkspaceAccessError extends Error {}
  return {
    RustApiError: MockRustApiError,
    WorkspaceAccessError: MockWorkspaceAccessError,
    rustApiResponseForAuthorizedWorkspace: vi.fn(),
    rustApiForAuthorizedWorkspace: vi.fn(),
    authorizedWorkspaceIdForRequest: vi.fn(async () => 'ws_test'),
  };
});

import { rustApiResponseForAuthorizedWorkspace } from '@/lib/server/tl-client';
import { POST } from './route';

const proxyMock = vi.mocked(rustApiResponseForAuthorizedWorkspace);

function postRequest(body: unknown): Request {
  return new Request('https://app.test/api/redteam/dispatch', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
}

const SUMMARY = {
  id: 'job_1',
  workspace_id: 'ws_test',
  environment_id: 'env',
  status: 'queued',
  target: 'http://127.0.0.1:9102',
  profile: 'fast',
  agent_id: null,
  attacks: 0,
  landed: 0,
  blocked: 0,
  error: null,
  created_at: '2026-06-13T00:00:00Z',
  updated_at: '2026-06-13T00:00:00Z',
};

describe('POST /api/redteam/dispatch', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies a valid loopback dispatch to the Rust orchestrator', async () => {
    proxyMock.mockResolvedValue({ data: SUMMARY, status: 201 });

    const res = await POST(postRequest({ target_url: 'http://127.0.0.1:9102', profile: 'fast' }));

    expect(res.status).toBe(201);
    expect(proxyMock).toHaveBeenCalledTimes(1);
    const [, path, init] = proxyMock.mock.calls[0] ?? [];
    expect(path).toBe('/v1/redteam/dispatch');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(String(init?.body))).toEqual({
      target_url: 'http://127.0.0.1:9102',
      profile: 'fast',
    });
  });

  it('rejects a non-loopback target before touching Rust', async () => {
    const res = await POST(postRequest({ target_url: 'http://10.0.0.5:9102', profile: 'fast' }));

    expect(res.status).toBe(400);
    expect(proxyMock).not.toHaveBeenCalled();
  });

  it('rejects an invalid profile', async () => {
    const res = await POST(postRequest({ target_url: 'http://127.0.0.1:9102', profile: 'turbo' }));

    expect(res.status).toBe(400);
    expect(proxyMock).not.toHaveBeenCalled();
  });

  it('rejects a malformed body', async () => {
    const res = await POST(
      new Request('https://app.test/api/redteam/dispatch', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: 'not json',
      }),
    );

    expect(res.status).toBe(400);
    expect(proxyMock).not.toHaveBeenCalled();
  });
});
