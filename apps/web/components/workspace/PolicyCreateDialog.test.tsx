import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { PolicyCreateDialog } from './PolicyCreateDialog';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ refresh: vi.fn() }),
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

describe('PolicyCreateDialog', () => {
  it('uses one create entrypoint with typed policy families underneath', async () => {
    render(
      <PolicyCreateDialog agents={[]} workspaceSlug="demo" contextQuery="?workspace=demo">
        New policy
      </PolicyCreateDialog>,
    );

    await userEvent.click(screen.getByRole('button', { name: /new policy/i }));

    expect(screen.getByRole('heading', { name: /create policy/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /protection policy/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /financial authorization/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /read the policy authoring guide/i })).toHaveAttribute(
      'href',
      '/docs/guides/policy-authoring',
    );

    await userEvent.click(screen.getByRole('button', { name: /financial authorization/i }));

    expect(screen.getByRole('heading', { name: /create financial policy/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /new financial policy/i })).not.toBeInTheDocument();
  });
});
