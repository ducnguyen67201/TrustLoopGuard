import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { AuthorizationApproval, AuthorizationGrantScope } from '@trustloopguard/sdk';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { AuthorizationApprovalsContent } from './AuthorizationApprovalsContent';

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

describe('AuthorizationApprovalsContent', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('shows only pending approvals and filters the one queue by domain', async () => {
    render(
      <AuthorizationApprovalsContent
        workspaceSlug="acme"
        environmentId="production"
        approvals={[
          approval('tool-pending', 'tool'),
          approval('finance-done', 'financial', 'approved'),
        ]}
      />,
    );

    expect(screen.getByText('tool:mail/send')).toBeInTheDocument();
    expect(screen.queryByText('financial:refund')).not.toBeInTheDocument();

    await userEvent.selectOptions(screen.getByRole('combobox', { name: /domain/i }), 'financial');
    expect(screen.getByText('No pending approvals')).toBeInTheDocument();
  });

  it('posts the reviewed envelope hash and exact proposed scope for scoped sign-off', async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(
        JSON.stringify({
          approval: { id: 'tool-pending', status: 'approved' },
          grant: { id: 'grant-1' },
        }),
        { status: 200 },
      ),
    );
    vi.stubGlobal('fetch', fetchMock);
    const reviewed = approval('tool-pending', 'tool');
    render(
      <AuthorizationApprovalsContent
        workspaceSlug="acme"
        environmentId="production"
        approvals={[reviewed]}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /review/i }));
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText('sha256:v1:reviewed')).toBeInTheDocument();
    await userEvent.click(
      within(dialog).getByRole('button', { name: /approve matching actions/i }),
    );

    expect(fetchMock).toHaveBeenCalledOnce();
    const call = fetchMock.mock.calls[0];
    expect(call).toBeDefined();
    const [, init] = call!;
    expect(JSON.parse(String(init?.body))).toEqual({
      decision: 'approve',
      mode: 'scoped',
      envelope_hash: 'sha256:v1:reviewed',
      scope: reviewed.envelope.proposed_scope,
    });
  });

  it('does not offer reusable sign-off when the envelope has no reviewed scope', async () => {
    render(
      <AuthorizationApprovalsContent
        workspaceSlug="acme"
        environmentId="production"
        approvals={[approval('exact', 'tool', 'pending', false)]}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /review/i }));
    expect(
      screen.queryByRole('button', { name: /approve matching actions/i }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /approve this action/i })).toBeInTheDocument();
  });
});

function approval(
  id: string,
  domain: 'tool' | 'financial',
  status: AuthorizationApproval['status'] = 'pending',
  reusable = true,
): AuthorizationApproval {
  const now = '2026-07-14T12:00:00Z';
  const proposedScope: AuthorizationGrantScope = {
    scope_type: 'action',
    scope: {
      operations: ['mail/send'],
      side_effects: ['external_communication'],
      parameters: { to: 'a@example.com' },
      allowed_destinations: ['a@example.com'],
    },
  };
  return {
    id,
    workspace_id: 'ws-1',
    environment_id: 'production',
    intent_id: `intent-${id}`,
    status,
    envelope: {
      schema: 'authorization-envelope:v1',
      intent_id: `intent-${id}`,
      domain,
      capability: domain === 'tool' ? 'tool:mail/send' : 'financial:refund',
      principal_id: 'agent-1',
      subject_id: id,
      subject_hash: 'sha256:v1:subject',
      exact_fingerprint: 'sha256:v1:exact',
      fingerprint_version: 1,
      requirement_ids: ['approval:send'],
      ...(reusable ? { proposed_scope: proposedScope } : {}),
      policy_versions: ['mail-policy:v1'],
      issued_at: now,
      expires_at: '2026-07-14T13:00:00Z',
    },
    envelope_hash: 'sha256:v1:reviewed',
    approver_roles: ['admin'],
    expires_at: '2026-07-14T13:00:00Z',
    created_at: now,
    updated_at: now,
  };
}
