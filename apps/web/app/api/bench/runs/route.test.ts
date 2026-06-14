import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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
import { GET, POST } from './route';

const proxyMock = vi.mocked(rustApiResponseForAuthorizedWorkspace);

function postRequest(body: unknown): Request {
  return new Request('https://app.test/api/bench/runs', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
}

const DETAIL = {
  run: {
    id: 'run_1',
    workspace_id: 'ws_test',
    environment_id: 'env',
    status: 'queued',
    profile: 'fast',
    generator: 'deterministic',
    agent_id: null,
    seed: null,
    error: null,
    created_at: '2026-06-14T00:00:00Z',
    updated_at: '2026-06-14T00:00:00Z',
  },
  arms: [],
  raw_job: null,
  guarded_job: null,
};

describe('/api/bench/runs', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('POST proxies a valid raw-vs-guarded loopback benchmark run', async () => {
    proxyMock.mockResolvedValue({ data: DETAIL, status: 201 });

    const res = await POST(
      postRequest({
        raw_target_url: 'http://127.0.0.1:9101',
        guarded_target_url: 'http://127.0.0.1:9102',
        profile: 'fast',
      }),
    );

    expect(res.status).toBe(201);
    const [, path, init] = proxyMock.mock.calls[0] ?? [];
    expect(path).toBe('/v1/bench/runs');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(String(init?.body))).toEqual({
      raw_target_url: 'http://127.0.0.1:9101',
      guarded_target_url: 'http://127.0.0.1:9102',
      profile: 'fast',
    });
  });

  it('POST rejects either non-loopback target before touching Rust', async () => {
    const res = await POST(
      postRequest({
        raw_target_url: 'https://evil.example.com',
        guarded_target_url: 'http://127.0.0.1:9102',
        profile: 'fast',
      }),
    );

    expect(res.status).toBe(400);
    expect(proxyMock).not.toHaveBeenCalled();
  });

  it('GET forwards list query params to Rust', async () => {
    proxyMock.mockResolvedValue({ data: { runs: [] }, status: 200 });

    const res = await GET(new Request('https://app.test/api/bench/runs?limit=5&workspace=ignore'));

    expect(res.status).toBe(200);
    const [, path] = proxyMock.mock.calls[0] ?? [];
    expect(path).toBe('/v1/bench/runs?limit=5');
  });
});
