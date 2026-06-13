import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/env', () => ({
  env: { REDTEAM_RUNNER_URL: 'http://127.0.0.1:8799' },
}));

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
  };
});

import { GET, POST } from './route';

const fetchMock = vi.fn<typeof fetch>();

function postRequest(body: unknown): Request {
  return new Request('https://app.test/api/arena/redteam', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
}

describe('/api/arena/redteam', () => {
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('proxies a valid run request to the runner with a timeout signal', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ runId: 'run_1', status: 'running' }), { status: 200 }),
    );

    const res = await POST(postRequest({ profile: 'fast', rawUrl: 'http://127.0.0.1:8787' }));

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe('http://127.0.0.1:8799/redteam/run');
    expect(init?.signal).toBeInstanceOf(AbortSignal);
    expect(res.status).toBe(200);
    await expect(res.json()).resolves.toEqual({ runId: 'run_1', status: 'running' });
  });

  it('rejects non-loopback targets before touching the runner', async () => {
    const res = await POST(postRequest({ profile: 'fast', rawUrl: 'http://10.0.0.5:8787' }));

    expect(res.status).toBe(400);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('maps a timed-out runner fetch to 504', async () => {
    fetchMock.mockRejectedValue(new DOMException('The operation timed out', 'TimeoutError'));

    const res = await POST(postRequest({ profile: 'fast' }));

    expect(res.status).toBe(504);
    await expect(res.json()).resolves.toEqual({
      error: 'red-team backend at http://127.0.0.1:8799 timed out',
    });
  });

  it('maps a refused connection to an actionable 503', async () => {
    fetchMock.mockRejectedValue(new TypeError('fetch failed'));

    const res = await POST(postRequest({ profile: 'fast' }));

    expect(res.status).toBe(503);
  });

  it('requires runId on poll requests', async () => {
    const res = await GET(new Request('https://app.test/api/arena/redteam'));

    expect(res.status).toBe(400);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('proxies poll requests to the runner with a timeout signal', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ runId: 'run_1', status: 'complete', report: null }), {
        status: 200,
      }),
    );

    const res = await GET(new Request('https://app.test/api/arena/redteam?runId=run_1'));

    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe('http://127.0.0.1:8799/redteam/runs/run_1');
    expect(init?.signal).toBeInstanceOf(AbortSignal);
    expect(res.status).toBe(200);
  });
});
