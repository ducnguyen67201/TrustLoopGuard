import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { McpAccessPageData } from '@/lib/server/dashboard-data';
import { McpAccessPageContent } from './McpAccessPageContent';

vi.mock('next/navigation', () => ({ useRouter: () => ({ refresh: vi.fn() }) }));

const base: McpAccessPageData = {
  user: { id: 'user', name: 'Member', email: 'member@example.com', avatar: '' },
  organization: { id: 'org', name: 'Org', slug: 'org' },
  activeWorkspace: { id: 'ws', name: 'Workspace', slug: 'workspace', description: '', policyCount: 0, enabledPolicies: 0, agentCount: 0, sourceCount: 0, apiKeyCount: 0, role: 'viewer', isKnowledgeBaseEnabled: false, isAttacksEnabled: false, isMcpGatewayEnabled: true },
  workspaces: [],
  activeEnvironment: { id: 'production', slug: 'production', name: 'Production', description: null, isDefault: true },
  environments: [], agents: [], isAdmin: false,
  connectInfo: { resource_url: 'https://guard.example/mcp', scope: 'mcp:tools', oauth_configured: true, default_environment_id: 'production', default_environment_name: 'Production' },
  connections: [], tools: [], members: [],
};

describe('McpAccessPageContent', () => {
  afterEach(cleanup);

  it('takes viewers directly to the shared managed connection without admin controls', () => {
    render(<McpAccessPageContent data={base} />);
    expect(screen.getByRole('heading', { level: 1, name: 'MCP Access' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Connect' })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Servers' })).not.toBeInTheDocument();
    expect(screen.getByDisplayValue('https://guard.example/mcp')).toBeInTheDocument();
  });

  it('shows the setup runway and admin workbenches for owners', () => {
    render(<McpAccessPageContent data={{ ...base, isAdmin: true, activeWorkspace: { ...base.activeWorkspace, role: 'owner' } }} />);
    expect(screen.getByRole('tab', { name: 'Overview' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Servers' })).toBeInTheDocument();
    expect(screen.getByText('Connect server')).toBeInTheDocument();
    expect(screen.getByText('Runtime policy')).toBeInTheDocument();
  });
});
