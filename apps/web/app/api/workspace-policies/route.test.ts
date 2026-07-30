import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/server/dashboard-data', () => ({
  getDashboardShell: vi.fn(),
}));
vi.mock('@/lib/server/tl-client', () => ({
  rustApiForUserWorkspace: vi.fn(),
}));

import { getDashboardShell, type DashboardShellData } from '@/lib/server/dashboard-data';
import { rustApiForUserWorkspace } from '@/lib/server/tl-client';
import { POST } from './route';

const shell: DashboardShellData = {
  isPlatformAdmin: false,
  user: { id: 'user-1', name: 'User', email: 'user@example.com', avatar: '' },
  organization: { id: 'org-1', name: 'Organization', slug: 'organization' },
  activeWorkspace: {
    id: 'ws-1',
    name: 'Workspace',
    slug: 'workspace',
    description: '',
    policyCount: 0,
    enabledPolicies: 0,
    agentCount: 0,
    sourceCount: 0,
    apiKeyCount: 0,
    role: 'viewer',
    isKnowledgeBaseEnabled: false,
    isAttacksEnabled: false,
    isMcpGatewayEnabled: false,
  },
  workspaces: [],
  activeEnvironment: {
    id: 'production',
    name: 'Production',
    slug: 'production',
    description: null,
    isDefault: true,
  },
  environments: [],
  agents: [],
};

function request(enabled = true): Request {
  return new Request('https://app.test/api/workspace-policies', {
    method: 'POST',
    body: JSON.stringify({
      workspace: 'workspace',
      enabled,
      draft: {
        id: 'block-secrets',
        description: 'Block secret disclosure',
        matchType: 'literal',
        matchValue: 'secret',
        action: 'deny',
        severity: 'high',
      },
    }),
  });
}

describe('/api/workspace-policies', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns 403 for a direct Viewer request before calling Rust', async () => {
    vi.mocked(getDashboardShell).mockResolvedValue(shell);

    const response = await POST(request());

    expect(response.status).toBe(403);
    await expect(response.json()).resolves.toEqual({
      error: 'workspace owner or admin role is required to create policies',
    });
    expect(rustApiForUserWorkspace).not.toHaveBeenCalled();
  });

  it.each(['owner', 'admin'])('forwards signed %s identity to Rust', async (role) => {
    vi.mocked(getDashboardShell).mockResolvedValue({
      ...shell,
      activeWorkspace: { ...shell.activeWorkspace, role },
    });
    vi.mocked(rustApiForUserWorkspace).mockResolvedValue(undefined);

    const response = await POST(request(false));

    expect(response.status).toBe(200);
    expect(rustApiForUserWorkspace).toHaveBeenNthCalledWith(
      1,
      shell.user,
      shell.activeWorkspace.id,
      '/v1/policies',
      expect.objectContaining({ method: 'POST' }),
    );
    expect(rustApiForUserWorkspace).toHaveBeenNthCalledWith(
      2,
      shell.user,
      shell.activeWorkspace.id,
      '/v1/policies/block-secrets/enabled',
      expect.objectContaining({ method: 'PATCH' }),
    );
  });
});
