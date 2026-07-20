import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  replace: vi.fn<(href: string) => void>(),
  refresh: vi.fn<() => void>(),
  success: vi.fn<(message: string) => void>(),
  error: vi.fn<(message: string) => void>(),
}));

vi.mock('next/navigation', () => ({
  useRouter: () => ({ replace: mocks.replace, refresh: mocks.refresh }),
}));
vi.mock('sonner', () => ({
  toast: { success: mocks.success, error: mocks.error },
}));

import { DeleteWorkspaceDialog } from './DeleteWorkspaceDialog';

const fetchMock = vi.fn<typeof fetch>();

function renderDialog(overrides: Partial<Parameters<typeof DeleteWorkspaceDialog>[0]> = {}) {
  render(
    <DeleteWorkspaceDialog
      workspaceId="ws_acme"
      workspaceName="Acme Support"
      workspaceRole="owner"
      isActive={false}
      fallbackWorkspaceSlug="other-team"
      {...overrides}
    />,
  );
}

async function openAndConfirm() {
  const user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'Delete Acme Support' }));
  const dialog = screen.getByRole('alertdialog');
  await user.type(within(dialog).getByLabelText('Type “Acme Support” to confirm'), 'Acme Support');
  return { user, dialog };
}

describe('DeleteWorkspaceDialog', () => {
  beforeEach(() => {
    fetchMock.mockReset();
    mocks.replace.mockReset();
    mocks.refresh.mockReset();
    mocks.success.mockReset();
    mocks.error.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it.each(['admin', 'editor', 'viewer'])('hides deletion from the %s role', (workspaceRole) => {
    renderDialog({ workspaceRole });
    expect(screen.queryByRole('button', { name: 'Delete Acme Support' })).not.toBeInTheDocument();
  });

  it('requires the exact case-sensitive and whitespace-sensitive workspace name', async () => {
    const user = userEvent.setup();
    renderDialog();

    await user.click(screen.getByRole('button', { name: 'Delete Acme Support' }));
    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toHaveTextContent('All members will lose access');
    expect(dialog).toHaveTextContent('Pending invitations will be revoked');
    expect(dialog).toHaveTextContent('Active runtime API keys will stop working');
    expect(dialog).toHaveTextContent('Historical guardrail records will be retained');
    expect(fetchMock).not.toHaveBeenCalled();

    const input = within(dialog).getByLabelText('Type “Acme Support” to confirm');
    const action = within(dialog).getByRole('button', { name: 'Delete workspace' });
    expect(action).toBeDisabled();

    for (const invalidName of ['Acme', 'acme support', ' Acme Support', 'Acme Support ']) {
      await user.clear(input);
      await user.type(input, invalidName);
      expect(action).toBeDisabled();
    }

    await user.clear(input);
    await user.type(input, 'Acme Support');
    expect(action).toBeEnabled();
  });

  it('allows only one request while deletion is in flight', async () => {
    let resolveResponse: (response: Response) => void = () => undefined;
    fetchMock.mockReturnValue(
      new Promise<Response>((resolve) => {
        resolveResponse = resolve;
      }),
    );
    renderDialog();
    const { dialog } = await openAndConfirm();
    const action = within(dialog).getByRole('button', { name: 'Delete workspace' });

    fireEvent.click(action);
    fireEvent.click(action);

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(action).toBeDisabled();
    await act(async () => resolveResponse(new Response(null, { status: 204 })));
    await waitFor(() => expect(mocks.refresh).toHaveBeenCalledTimes(1));
  });

  it('refreshes in place after deleting an inactive workspace', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    renderDialog();
    const { user, dialog } = await openAndConfirm();

    await user.click(within(dialog).getByRole('button', { name: 'Delete workspace' }));

    await waitFor(() => expect(mocks.refresh).toHaveBeenCalledTimes(1));
    expect(mocks.replace).not.toHaveBeenCalled();
    expect(mocks.success).toHaveBeenCalledWith('Workspace “Acme Support” deleted');
    expect(fetchMock).toHaveBeenCalledWith('/api/me/workspaces/ws_acme', {
      method: 'DELETE',
      headers: expect.any(Headers),
    });
  });

  it('selects another workspace after deleting the active workspace', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    renderDialog({ isActive: true, fallbackWorkspaceSlug: 'other/team' });
    const { user, dialog } = await openAndConfirm();

    await user.click(within(dialog).getByRole('button', { name: 'Delete workspace' }));

    await waitFor(() => {
      expect(mocks.replace).toHaveBeenCalledWith('/workspaces?workspace=other%2Fteam');
    });
    expect(mocks.refresh).toHaveBeenCalledTimes(1);
  });

  it('sends the owner to onboarding after deleting the last workspace', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    renderDialog({ isActive: true, fallbackWorkspaceSlug: null });
    const { user, dialog } = await openAndConfirm();

    await user.click(within(dialog).getByRole('button', { name: 'Delete workspace' }));

    await waitFor(() => expect(mocks.replace).toHaveBeenCalledWith('/onboarding/workspace'));
    expect(mocks.refresh).not.toHaveBeenCalled();
  });

  it('keeps the dialog and confirmation available after a useful failure', async () => {
    fetchMock.mockResolvedValue(
      new Response(
        JSON.stringify({ code: 'forbidden', message: 'Only owners can delete this workspace' }),
        { status: 403, headers: { 'content-type': 'application/json' } },
      ),
    );
    renderDialog();
    const { user, dialog } = await openAndConfirm();
    const action = within(dialog).getByRole('button', { name: 'Delete workspace' });

    await user.click(action);

    await waitFor(() => {
      expect(mocks.error).toHaveBeenCalledWith('Only owners can delete this workspace');
    });
    expect(screen.getByRole('alertdialog')).toBeInTheDocument();
    expect(within(dialog).getByLabelText('Type “Acme Support” to confirm')).toHaveValue(
      'Acme Support',
    );
    expect(action).toBeEnabled();
    expect(mocks.success).not.toHaveBeenCalled();
    expect(mocks.replace).not.toHaveBeenCalled();
    expect(mocks.refresh).not.toHaveBeenCalled();
  });
});
