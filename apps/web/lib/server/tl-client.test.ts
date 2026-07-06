import { beforeEach, describe, expect, it, vi } from 'vitest';

const mockState = vi.hoisted(() => ({
  auth: vi.fn<() => Promise<{ user?: { id?: string; email?: string; tlJwt?: string } } | null>>(),
  env: {
    TL_API_KEY: 'internal-service-key',
    TL_SERVER_URL: 'https://rust.test',
  },
}));

vi.mock('server-only', () => ({}));
vi.mock('@trustloopguard/sdk', () => ({
  Client: class MockClient {},
}));
vi.mock('@/auth', () => ({
  auth: mockState.auth,
}));
vi.mock('@/env', () => ({
  env: mockState.env,
}));

import {
  isUserApprovalRequiredError,
  RustApiError,
  rustApiForAuthorizedWorkspace,
  rustApiForUserWorkspace,
} from './tl-client';
import { selectAuthorizedWorkspaceId } from '../workspace-access';

const memberships = [
  {
    id: 'ws_alpha',
    slug: 'alpha',
  },
  {
    id: 'ws_beta',
    slug: 'beta-team',
  },
];

describe('selectAuthorizedWorkspaceId', () => {
  it('falls back to the first membership when no workspace is requested', () => {
    expect(selectAuthorizedWorkspaceId(memberships, null)).toBe('ws_alpha');
  });

  it('resolves requested workspace slugs and ids through memberships', () => {
    expect(selectAuthorizedWorkspaceId(memberships, 'beta-team')).toBe('ws_beta');
    expect(selectAuthorizedWorkspaceId(memberships, 'ws_beta')).toBe('ws_beta');
  });

  it('rejects workspaces outside the membership list', () => {
    expect(selectAuthorizedWorkspaceId(memberships, 'ws_not_member')).toBeNull();
    expect(selectAuthorizedWorkspaceId([], 'alpha')).toBeNull();
  });
});

describe('isUserApprovalRequiredError', () => {
  it('detects approval denials from Rust', () => {
    expect(
      isUserApprovalRequiredError(
        new RustApiError(
          '/v1/team/my-workspaces',
          403,
          'user is not approved',
        ),
      ),
    ).toBe(true);
    expect(
      isUserApprovalRequiredError(
        new RustApiError('/v1/team/my-workspaces', 403, 'workspace access denied'),
      ),
    ).toBe(false);
  });
});

describe('tl-client Rust auth forwarding', () => {
  const fetchMock = vi.fn<typeof fetch>();

  beforeEach(() => {
    fetchMock.mockReset();
    mockState.auth.mockReset();
    mockState.env.TL_API_KEY = 'internal-service-key';
    vi.stubGlobal('fetch', fetchMock);
  });

  it('forwards workspace and dashboard user identity for user-scoped workspace calls', async () => {
    fetchMock.mockResolvedValue(new Response(JSON.stringify({ api_keys: [] })));

    await rustApiForUserWorkspace(
      { id: '00000000-0000-0000-0000-000000000001', email: 'owner@example.com' },
      'ws_acme',
      '/v1/api-keys',
    );

    expect(fetchMock).toHaveBeenCalledWith('https://rust.test/v1/api-keys', {
      headers: expect.any(Headers),
    });
    const headers = headersForCall(fetchMock, 0);
    expect(headers.get('authorization')).toBe('Bearer internal-service-key');
    expect(headers.get('x-tlg-workspace-id')).toBe('ws_acme');
    expect(headers.get('x-tlg-user-id')).toBe('00000000-0000-0000-0000-000000000001');
    expect(headers.get('x-tlg-user-email')).toBe('owner@example.com');
  });

  it('uses internal auth for workspace calls even when the session has a Rust JWT', async () => {
    fetchMock.mockResolvedValue(new Response(JSON.stringify({ api_keys: [] })));

    await rustApiForUserWorkspace(
      {
        id: '00000000-0000-0000-0000-000000000001',
        email: 'owner@example.com',
        tlJwt: 'user-session-jwt',
      },
      'ws_acme',
      '/v1/team/members',
    );

    const headers = headersForCall(fetchMock, 0);
    expect(headers.get('authorization')).toBe('Bearer internal-service-key');
    expect(headers.get('x-tlg-workspace-id')).toBe('ws_acme');
    expect(headers.get('x-tlg-user-id')).toBe('00000000-0000-0000-0000-000000000001');
    expect(headers.get('x-tlg-user-email')).toBe('owner@example.com');
  });

  it('uses the request session user when proxying an authorized workspace request', async () => {
    mockState.auth.mockResolvedValue({
      user: {
        id: '00000000-0000-0000-0000-000000000002',
        email: 'admin@example.com',
      },
    });
    fetchMock
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            workspaces: [{ id: 'ws_acme', slug: 'acme' }],
          }),
        ),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({ api_key: { id: 'key_1' } })));

    await rustApiForAuthorizedWorkspace(
      new Request('https://app.test/api/api-keys?workspace=acme'),
      '/v1/api-keys',
      { method: 'POST' },
    );

    const apiKeyHeaders = headersForCall(fetchMock, 1);
    expect(apiKeyHeaders.get('authorization')).toBe('Bearer internal-service-key');
    expect(apiKeyHeaders.get('x-tlg-workspace-id')).toBe('ws_acme');
    expect(apiKeyHeaders.get('x-tlg-user-id')).toBe('00000000-0000-0000-0000-000000000002');
    expect(apiKeyHeaders.get('x-tlg-user-email')).toBe('admin@example.com');
  });

  it('uses internal auth when resolving workspace access even if the session JWT is stale', async () => {
    mockState.auth.mockResolvedValue({
      user: {
        id: '00000000-0000-0000-0000-000000000002',
        email: 'admin@example.com',
        tlJwt: 'stale-user-session-jwt',
      },
    });
    fetchMock
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            workspaces: [{ id: 'ws_acme', slug: 'acme' }],
          }),
        ),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({ api_key: { id: 'key_1' } })));

    await rustApiForAuthorizedWorkspace(
      new Request('https://app.test/api/api-keys?workspace=acme'),
      '/v1/api-keys',
      { method: 'POST' },
    );

    const workspaceLookupHeaders = headersForCall(fetchMock, 0);
    expect(workspaceLookupHeaders.get('authorization')).toBe('Bearer internal-service-key');
    expect(workspaceLookupHeaders.get('x-tlg-user-id')).toBe(
      '00000000-0000-0000-0000-000000000002',
    );
    expect(workspaceLookupHeaders.get('x-tlg-user-email')).toBe('admin@example.com');
  });
});

function headersForCall(fetchMock: ReturnType<typeof vi.fn<typeof fetch>>, index: number): Headers {
  const init = fetchMock.mock.calls[index]?.[1];
  if (init === undefined || !(init.headers instanceof Headers)) {
    throw new Error(`fetch call ${index} did not use Headers`);
  }
  return init.headers;
}
