import { beforeEach, describe, expect, it, vi } from 'vitest';

interface SessionUser {
  id?: string;
  email?: string | null;
  tlJwt?: string;
}

type AuthMock = () => Promise<{ user?: SessionUser } | null>;
type DeleteWorkspaceFromRust = (
  user: { id: string; email?: string | null; tlJwt?: string },
  path: string,
  init?: RequestInit,
) => Promise<void>;

const mocks = vi.hoisted(() => ({
  auth: vi.fn<AuthMock>(),
  rustApiForUser: vi.fn<DeleteWorkspaceFromRust>(),
}));

vi.mock('@/auth', () => ({ auth: mocks.auth }));
vi.mock('@/lib/server/tl-client', () => {
  class MockRustApiError extends Error {
    constructor(
      public readonly path: string,
      public readonly status: number,
      public readonly body: string,
    ) {
      super(`Rust API ${path} failed with ${status}: ${body}`);
    }
  }

  class MockWorkspaceAccessError extends Error {}

  return {
    RustApiError: MockRustApiError,
    WorkspaceAccessError: MockWorkspaceAccessError,
    rustApiForUser: mocks.rustApiForUser,
    rustApiForAuthorizedWorkspace: vi.fn(),
    rustApiResponseForAuthorizedWorkspace: vi.fn(),
  };
});

import { RustApiError } from '@/lib/server/tl-client';
import { DELETE } from './route';

function context(id = 'ws/acme') {
  return { params: Promise.resolve({ id }) };
}

describe('DELETE /api/me/workspaces/[id]', () => {
  beforeEach(() => {
    mocks.auth.mockReset();
    mocks.rustApiForUser.mockReset();
  });

  it('proxies an authenticated user-scoped delete and returns no content', async () => {
    mocks.auth.mockResolvedValue({
      user: {
        id: '00000000-0000-0000-0000-000000000001',
        email: 'owner@example.com',
        tlJwt: 'user-jwt',
      },
    });
    mocks.rustApiForUser.mockResolvedValue();

    const response = await DELETE(
      new Request('https://app.test/api/me/workspaces/ws%2Facme', {
        method: 'DELETE',
      }),
      context(),
    );

    expect(response.status).toBe(204);
    expect(await response.text()).toBe('');
    expect(mocks.rustApiForUser).toHaveBeenCalledTimes(1);
    expect(mocks.rustApiForUser).toHaveBeenCalledWith(
      {
        id: '00000000-0000-0000-0000-000000000001',
        email: 'owner@example.com',
        tlJwt: 'user-jwt',
      },
      '/v1/team/my-workspaces/ws%2Facme',
      { method: 'DELETE' },
    );
  });

  it('rejects an unauthenticated request without calling Rust', async () => {
    mocks.auth.mockResolvedValue(null);

    const response = await DELETE(
      new Request('https://app.test/api/me/workspaces/ws_acme', {
        method: 'DELETE',
      }),
      context('ws_acme'),
    );

    expect(response.status).toBe(401);
    expect(await response.json()).toEqual({ error: 'Unauthorized' });
    expect(mocks.rustApiForUser).not.toHaveBeenCalled();
  });

  it.each([
    [403, 'forbidden'],
    [404, 'not_found'],
  ])('preserves a structured Rust %i response', async (status, code) => {
    const body = JSON.stringify({ code, message: 'workspace request denied' });
    mocks.auth.mockResolvedValue({
      user: { id: '00000000-0000-0000-0000-000000000001' },
    });
    mocks.rustApiForUser.mockRejectedValue(
      new RustApiError('/v1/team/my-workspaces/ws_acme', status, body),
    );

    const response = await DELETE(
      new Request('https://app.test/api/me/workspaces/ws_acme', {
        method: 'DELETE',
      }),
      context('ws_acme'),
    );

    expect(response.status).toBe(status);
    expect(await response.json()).toEqual({
      code,
      message: 'workspace request denied',
    });
    expect(response.headers.get('content-type')).toContain('application/json');
  });

  it('hides generic upstream failure details', async () => {
    mocks.auth.mockResolvedValue({
      user: { id: '00000000-0000-0000-0000-000000000001' },
    });
    mocks.rustApiForUser.mockRejectedValue(new Error('private upstream host failed'));

    const response = await DELETE(
      new Request('https://app.test/api/me/workspaces/ws_acme', {
        method: 'DELETE',
      }),
      context('ws_acme'),
    );

    expect(response.status).toBe(502);
    expect(await response.json()).toEqual({ error: 'upstream request failed' });
  });
});
