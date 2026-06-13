import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/env', () => ({
  env: { REDTEAM_RUNNER_URL: 'http://127.0.0.1:8799' },
}));

const authState = vi.hoisted(() => ({
  authorize: vi.fn<(req: Request) => Promise<string>>(),
}));

vi.mock('@/lib/server/tl-client', () => {
  class MockRustApiError extends Error {}
  class MockWorkspaceAccessError extends Error {
    constructor(
      public readonly status: number,
      message: string,
    ) {
      super(message);
    }
  }
  return {
    RustApiError: MockRustApiError,
    WorkspaceAccessError: MockWorkspaceAccessError,
    rustApiResponseForAuthorizedWorkspace: vi.fn(),
    rustApiForAuthorizedWorkspace: vi.fn(),
    authorizedWorkspaceIdForRequest: (req: Request) => authState.authorize(req),
  };
});

import { WorkspaceAccessError } from '@/lib/server/tl-client';

import { GET, POST } from './route';

const fetchMock = vi.fn<typeof fetch>();
const TARGET = 'http://127.0.0.1:9102';

function postRequest(body: unknown): Request {
  return new Request('https://app.test/api/attacks', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
}

describe('/api/attacks', () => {
  beforeEach(() => {
    fetchMock.mockReset();
    authState.authorize.mockReset();
    authState.authorize.mockResolvedValue('ws_test');
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('forwards a single-target attack to the runner', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ runId: 'run_1', status: 'running' }), { status: 200 }),
    );

    const res = await POST(postRequest({ profile: 'fast', targetUrl: TARGET }));

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe('http://127.0.0.1:8799/redteam/run');
    expect(JSON.parse(String(init?.body))).toEqual({ profile: 'fast', targetUrl: TARGET });
    expect(res.status).toBe(200);
  });

  it('rejects a non-loopback target before touching the runner', async () => {
    const res = await POST(postRequest({ profile: 'fast', targetUrl: 'http://10.0.0.5:9102' }));

    expect(res.status).toBe(400);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('returns 401 when the request is not authorized', async () => {
    authState.authorize.mockRejectedValue(new WorkspaceAccessError(401, 'unauthorized'));

    const res = await POST(postRequest({ profile: 'fast', targetUrl: TARGET }));

    expect(res.status).toBe(401);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('requires runId on poll requests', async () => {
    const res = await GET(new Request('https://app.test/api/attacks'));

    expect(res.status).toBe(400);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
