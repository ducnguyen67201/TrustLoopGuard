import { selectAuthorizedWorkspaceId } from '../workspace-access';
import { describe, expect, it } from 'vitest';

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
