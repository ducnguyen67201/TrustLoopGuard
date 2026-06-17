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
      mode: 'one_off',
      attack_surface: 'chat',
    });
  });

  it('proxies learning mode to the Rust orchestrator', async () => {
    proxyMock.mockResolvedValue({ data: SUMMARY, status: 201 });

    const res = await POST(
      postRequest({
        target_url: 'http://127.0.0.1:9102',
        profile: 'fast',
        mode: 'learning',
      }),
    );

    expect(res.status).toBe(201);
    const [, , init] = proxyMock.mock.calls[0] ?? [];
    expect(JSON.parse(String(init?.body))).toMatchObject({ mode: 'learning' });
  });

  it('proxies document workflow attack surface to the Rust orchestrator', async () => {
    proxyMock.mockResolvedValue({ data: SUMMARY, status: 201 });

    const res = await POST(
      postRequest({
        target_url: 'http://127.0.0.1:9102',
        profile: 'fast',
        attack_surface: 'document_workflow',
      }),
    );

    expect(res.status).toBe(201);
    const [, , init] = proxyMock.mock.calls[0] ?? [];
    expect(JSON.parse(String(init?.body))).toMatchObject({
      attack_surface: 'document_workflow',
    });
  });

  it('proxies a document workflow PDF template without manual fields to the Rust orchestrator', async () => {
    proxyMock.mockResolvedValue({ data: SUMMARY, status: 201 });

    const res = await POST(
      postRequest({
        target_url: 'http://127.0.0.1:9102',
        profile: 'fast',
        attack_surface: 'document_workflow',
        document_template: {
          file_name: 'form.pdf',
          media_type: 'application/pdf',
          data_base64: 'JVBERi0xLjQK',
          flatten: false,
        },
      }),
    );

    expect(res.status).toBe(201);
    const [, , init] = proxyMock.mock.calls[0] ?? [];
    expect(JSON.parse(String(init?.body))).toMatchObject({
      attack_surface: 'document_workflow',
      document_template: {
        file_name: 'form.pdf',
        media_type: 'application/pdf',
        data_base64: 'JVBERi0xLjQK',
        flatten: false,
      },
    });
  });

  it('rejects a PDF template on chat dispatches', async () => {
    const res = await POST(
      postRequest({
        target_url: 'http://127.0.0.1:9102',
        profile: 'fast',
        document_template: {
          file_name: 'form.pdf',
          media_type: 'application/pdf',
          data_base64: 'JVBERi0xLjQK',
          fields: { 'field.name': '{case_body}' },
        },
      }),
    );

    expect(res.status).toBe(400);
    expect(proxyMock).not.toHaveBeenCalled();
  });

  it('rejects document templates that are not PDF bytes', async () => {
    const res = await POST(
      postRequest({
        target_url: 'http://127.0.0.1:9102',
        profile: 'fast',
        attack_surface: 'document_workflow',
        document_template: {
          file_name: 'form.pdf',
          media_type: 'application/pdf',
          data_base64: 'bm90LXBkZg==',
          fields: { 'field.name': '{case_body}' },
        },
      }),
    );

    expect(res.status).toBe(400);
    expect(proxyMock).not.toHaveBeenCalled();
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

  it('rejects an invalid run mode', async () => {
    const res = await POST(
      postRequest({ target_url: 'http://127.0.0.1:9102', profile: 'fast', mode: 'forever' }),
    );

    expect(res.status).toBe(400);
    expect(proxyMock).not.toHaveBeenCalled();
  });

  it('rejects an invalid attack surface', async () => {
    const res = await POST(
      postRequest({
        target_url: 'http://127.0.0.1:9102',
        profile: 'fast',
        attack_surface: 'email_campaign',
      }),
    );

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
