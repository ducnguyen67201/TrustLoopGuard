import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { AuthorizationGrant } from '@trustloopguard/sdk';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { AuthorizationGrantsContent } from './AuthorizationGrantsContent';

const refresh = vi.fn();

vi.mock('next/navigation', () => ({
  useRouter: () => ({ refresh }),
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

describe('AuthorizationGrantsContent', () => {
  afterEach(() => {
    cleanup();
    refresh.mockReset();
    vi.unstubAllGlobals();
  });

  it('creates typed, requirement-bound tool authority', async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValue(
        new Response(JSON.stringify({ id: 'grant-new', status: 'active' }), { status: 201 }),
      );
    vi.stubGlobal('fetch', fetchMock);
    render(
      <AuthorizationGrantsContent workspaceSlug="acme" environmentId="production" grants={[]} />,
    );

    await userEvent.click(screen.getByRole('button', { name: /create grant/i }));
    await userEvent.type(screen.getByLabelText('Principal'), 'agent-1');
    await userEvent.type(screen.getByLabelText('Capability'), 'mail/send');
    await userEvent.type(screen.getByLabelText(/requirement ids/i), 'approval:mail/send');
    await userEvent.type(screen.getByLabelText('Operation'), 'mail/send');
    await userEvent.type(screen.getByLabelText(/maximum uses/i), '5');
    await userEvent.type(screen.getByLabelText('Server ID'), 'mail');
    await userEvent.type(screen.getByLabelText('Tool name'), 'send');
    await userEvent.click(screen.getByRole('button', { name: /create grant/i }));

    expect(fetchMock).toHaveBeenCalledOnce();
    const call = fetchMock.mock.calls[0];
    expect(call).toBeDefined();
    const [url, init] = call!;
    expect(url).toBe('/api/authorization/grants?workspace=acme&environment=production');
    expect(JSON.parse(String(init?.body))).toEqual({
      principal_id: 'agent-1',
      domain: 'tool',
      capability: 'tool:mail/send',
      requirement_ids: ['approval:mail/send'],
      max_uses: 5,
      scope: {
        scope_type: 'action',
        scope: {
          operations: ['mail/send'],
          side_effects: ['api_mutation'],
          server_id: 'mail',
          tool_name: 'send',
          parameters: {},
          allowed_destinations: [],
        },
      },
    });
    expect(refresh).toHaveBeenCalledOnce();
  });

  it('revokes active grants through the common grant endpoint', async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValue(
        new Response(JSON.stringify({ id: 'grant-1', status: 'revoked' }), { status: 200 }),
      );
    vi.stubGlobal('fetch', fetchMock);
    render(
      <AuthorizationGrantsContent
        workspaceSlug="acme"
        environmentId="production"
        grants={[grant()]}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /revoke/i }));

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/authorization/grants/grant-1/revoke?workspace=acme&environment=production',
      { method: 'POST' },
    );
    expect(await screen.findByText('revoked')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /revoke/i })).not.toBeInTheDocument();
  });
});

function grant(): AuthorizationGrant {
  const now = '2026-07-14T12:00:00Z';
  return {
    id: 'grant-1',
    workspace_id: 'ws-1',
    environment_id: 'production',
    principal_id: 'agent-1',
    domain: 'tool',
    capability: 'tool:mail/send',
    mode: 'scoped',
    status: 'active',
    source: 'user_intent',
    scope: {
      scope_type: 'action',
      scope: {
        operations: ['mail/send'],
        side_effects: ['external_communication'],
        parameters: { to: 'a@example.com' },
        allowed_destinations: ['a@example.com'],
      },
    },
    fingerprint_version: 1,
    requirement_ids: ['approval:mail/send'],
    max_uses: 5,
    use_count: 0,
    created_by: 'user-1',
    created_at: now,
    updated_at: now,
  };
}
