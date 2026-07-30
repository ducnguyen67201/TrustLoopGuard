import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('next/cache', () => ({
  revalidatePath: vi.fn(),
}));
vi.mock('next/navigation', () => ({
  redirect: vi.fn(),
}));
vi.mock('@/lib/server/dashboard-data', () => ({
  getDashboardShell: vi.fn(),
}));
vi.mock('@/lib/server/tl-client', () => ({
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
import { createKnowledgeSource } from './actions';

const shell: DashboardShellData = {
  isPlatformAdmin: false,
  user: {
    id: 'user-1',
    name: 'User',
    email: 'user@example.com',
    avatar: '',
  },
  organization: {
    id: 'org-1',
    name: 'Organization',
    slug: 'organization',
  },
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
    isKnowledgeBaseEnabled: true,
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

function sourceForm(): FormData {
  const form = new FormData();
  form.set('workspaceSlug', 'workspace');
  form.set('title', 'Runbook');
  form.set('kind', 'note');
  form.set('notes', 'Trusted operating instructions');
  return form;
}

describe('createKnowledgeSource', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('rejects direct Viewer invocation before calling Rust', async () => {
    vi.mocked(getDashboardShell).mockResolvedValue(shell);

    await expect(createKnowledgeSource(sourceForm())).rejects.toMatchObject({
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

    await createKnowledgeSource(sourceForm());

    expect(rustApiForUserWorkspace).toHaveBeenCalledWith(
      shell.user,
      shell.activeWorkspace.id,
      '/v1/knowledge-sources',
      expect.objectContaining({ method: 'POST' }),
    );
  });
});
