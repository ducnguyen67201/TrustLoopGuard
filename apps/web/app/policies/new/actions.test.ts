import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('next/cache', () => ({ revalidatePath: vi.fn() }));
vi.mock('next/navigation', () => ({ redirect: vi.fn() }));
vi.mock('@/lib/server/dashboard-data', () => ({ getDashboardShell: vi.fn() }));
vi.mock('@/lib/server/tl-client', () => ({
  RustApiError: class extends Error {
    constructor(
      public readonly path: string,
      public readonly status: number,
      public readonly body: string,
    ) {
      super(body);
    }
  },
  WorkspaceAccessError: class extends Error {
    constructor(
      public readonly status: 401 | 403,
      message: string,
    ) {
      super(message);
    }
  },
  rustApiForUserWorkspace: vi.fn(),
}));

import { getDashboardShell, type DashboardShellData } from '@/lib/server/dashboard-data';
import { rustApiForUserWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';
import { createPolicy } from './actions';

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

function policyForm(): FormData {
  const form = new FormData();
  form.set('workspaceSlug', 'workspace');
  form.set('policyKey', 'block-secrets');
  form.set('description', 'Block secret disclosure');
  form.set('severity', 'high');
  form.set('action', 'deny');
  form.set('enabled', 'true');
  return form;
}

describe('createPolicy', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('rejects direct Viewer invocation before calling Rust', async () => {
    vi.mocked(getDashboardShell).mockResolvedValue(shell);

    await expect(createPolicy({}, policyForm())).rejects.toMatchObject({
      status: 403,
    } satisfies Partial<WorkspaceAccessError>);
    expect(rustApiForUserWorkspace).not.toHaveBeenCalled();
  });

  it.each(['owner', 'admin'])('forwards signed %s identity to Rust', async (role) => {
    vi.mocked(getDashboardShell).mockResolvedValue({
      ...shell,
      activeWorkspace: { ...shell.activeWorkspace, role },
    });
    vi.mocked(rustApiForUserWorkspace).mockResolvedValue(undefined);

    await createPolicy({}, policyForm());

    expect(rustApiForUserWorkspace).toHaveBeenCalledWith(
      shell.user,
      shell.activeWorkspace.id,
      '/v1/policies',
      expect.objectContaining({ method: 'POST' }),
    );
  });
});
