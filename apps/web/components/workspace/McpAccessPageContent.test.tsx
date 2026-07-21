import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { McpAccessPageData } from '@/lib/server/dashboard-data';
import { McpAccessPageContent } from './McpAccessPageContent';

vi.mock('next/navigation', () => ({ useRouter: () => ({ refresh: vi.fn() }) }));

const base: McpAccessPageData = {
  user: { id: 'user', name: 'Member', email: 'member@example.com', avatar: '' },
  organization: { id: 'org', name: 'Org', slug: 'org' },
  activeWorkspace: {
    id: 'ws',
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
    isMcpGatewayEnabled: true,
  },
  workspaces: [],
  activeEnvironment: {
    id: 'production',
    slug: 'production',
    name: 'Production',
    description: null,
    isDefault: true,
  },
  environments: [],
  agents: [],
  isAdmin: false,
  connectInfo: {
    resource_url: 'https://guard.example/mcp',
    scope: 'mcp:tools',
    oauth_configured: true,
    default_environment_id: 'production',
    default_environment_name: 'Production',
  },
  connections: [],
  tools: [],
  members: [],
};

describe('McpAccessPageContent', () => {
  afterEach(cleanup);

  it('takes viewers directly to the shared managed connection without admin controls', () => {
    render(<McpAccessPageContent data={base} />);
    expect(screen.getByRole('heading', { level: 1, name: 'MCP Access' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Connect' })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Servers' })).not.toBeInTheDocument();
    expect(screen.getByDisplayValue('https://guard.example/mcp')).toBeInTheDocument();
    expect(screen.getByLabelText('Remote MCP endpoint')).toHaveValue('https://guard.example/mcp');
  });

  it('shows the setup runway and admin workbenches for owners', () => {
    render(
      <McpAccessPageContent
        data={{
          ...base,
          isAdmin: true,
          activeWorkspace: { ...base.activeWorkspace, role: 'owner' },
        }}
      />,
    );
    expect(screen.getByRole('tab', { name: 'Overview' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Servers' })).toBeInTheDocument();
    expect(screen.getByText('Connect server')).toBeInTheDocument();
    expect(screen.getByText('Runtime policy')).toBeInTheDocument();
  });

  it('clears a write-only credential after connecting a server', async () => {
    const user = userEvent.setup();
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ id: 'connection' }), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ tool_count: 1 }), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    render(
      <McpAccessPageContent
        data={{
          ...base,
          isAdmin: true,
          activeWorkspace: { ...base.activeWorkspace, role: 'owner' },
        }}
      />,
    );

    await user.click(screen.getByRole('tab', { name: 'Servers' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Display name')).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Connect server' }));
    expect(screen.getByRole('dialog', { name: 'Connect an MCP server' })).toBeInTheDocument();
    await user.type(screen.getByLabelText('Display name'), 'Company tools');
    await user.type(screen.getByLabelText('Stable slug'), 'company');
    await user.type(screen.getByLabelText('HTTPS endpoint'), 'https://tools.example/mcp');
    const credential = screen.getByLabelText('Bearer token (optional)');
    await user.type(credential, 'secret-value');
    await user.click(screen.getByRole('button', { name: 'Connect and sync' }));

    await waitFor(() => expect(credential).toHaveValue(''));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('presents connected servers as a compact fleet with all server actions', async () => {
    const user = userEvent.setup();
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({}), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const admin = {
      ...base,
      isAdmin: true,
      activeWorkspace: { ...base.activeWorkspace, role: 'owner' as const },
      connections: [
        {
          id: 'connection',
          display_name: 'Company tools',
          server_slug: 'company',
          endpoint_url: 'https://tools.example/mcp',
          auth_kind: 'static_bearer' as const,
          credential_status: 'configured' as const,
          enabled: true,
          last_sync_status: 'succeeded' as const,
          last_synced_at: '2026-07-19T16:30:00Z',
          tool_count: 12,
          created_at: '2026-07-19T00:00:00Z',
          updated_at: '2026-07-19T16:30:00Z',
        },
      ],
    };
    render(<McpAccessPageContent data={admin} />);

    await user.click(screen.getByRole('tab', { name: 'Servers' }));

    expect(screen.getByText('Server fleet')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Connected MCP servers' })).toBeInTheDocument();
    expect(screen.getByText('Company tools')).toBeInTheDocument();
    expect(screen.getByText('Bearer secured')).toBeInTheDocument();
    expect(screen.getByText('Succeeded')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sync Company tools' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Disable Company tools' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Delete Company tools' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Sync Company tools' }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('/api/mcp-gateway/connections/connection/sync'),
        expect.objectContaining({ method: 'POST' }),
      ),
    );
  });

  it('labels member selection and lets admins classify a tool side effect', async () => {
    const user = userEvent.setup();
    Element.prototype.scrollIntoView = vi.fn();
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({}), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const admin = {
      ...base,
      isAdmin: true,
      activeWorkspace: { ...base.activeWorkspace, role: 'owner' as const },
      agents: [{ id: 'agent-customer', name: 'Customer agent' }],
      members: [{ user_id: 'intern', username: 'leo_intern', role: 'viewer' as const }],
      tools: [
        {
          id: 'tool',
          connection_id: 'connection',
          connection_name: 'Company tools',
          upstream_name: 'charge',
          public_name: 'company__charge',
          input_schema: {},
          annotations: {},
          schema_hash: 'hash',
          side_effect: 'read' as const,
          catalog_status: 'active' as const,
          assigned_user_ids: [],
          agent_assignments: [],
          unbound_user_ids: [],
          created_at: '2026-07-19T00:00:00Z',
          updated_at: '2026-07-19T00:00:00Z',
        },
      ],
    };
    render(<McpAccessPageContent data={admin} />);

    await user.click(screen.getByRole('tab', { name: 'Tool access' }));
    expect(screen.getByLabelText('Member')).toBeInTheDocument();
    expect(screen.getByLabelText('Agent')).toBeInTheDocument();
    fireEvent.keyDown(screen.getByRole('combobox', { name: 'Classify company__charge' }), {
      key: 'ArrowDown',
    });
    await user.click(screen.getByRole('option', { name: 'API mutation' }));

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('/api/mcp-gateway/tools/tool'),
        expect.objectContaining({
          method: 'PATCH',
          body: JSON.stringify({ side_effect: 'api_mutation' }),
        }),
      ),
    );
  });

  it('grants only the selected member and agent pair and identifies legacy access', async () => {
    const user = userEvent.setup();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValue(new Response(JSON.stringify({}), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const admin: McpAccessPageData = {
      ...base,
      isAdmin: true,
      activeWorkspace: { ...base.activeWorkspace, role: 'owner' },
      agents: [
        { id: 'agent-customer', name: 'Customer agent' },
        { id: 'agent-finance', name: 'Finance agent' },
      ],
      members: [{ user_id: 'intern', username: 'leo_intern', role: 'viewer' }],
      tools: [
        {
          id: 'tool',
          connection_id: 'connection',
          connection_name: 'Company tools',
          upstream_name: 'customer_database_query',
          public_name: 'company__customer_database_query',
          input_schema: {},
          annotations: {},
          schema_hash: 'hash',
          side_effect: 'read',
          catalog_status: 'active',
          assigned_user_ids: ['legacy-user'],
          agent_assignments: [{ user_id: 'finance-user', agent_id: 'agent-customer' }],
          unbound_user_ids: ['legacy-user'],
          created_at: '2026-07-19T00:00:00Z',
          updated_at: '2026-07-19T00:00:00Z',
        },
      ],
    };
    render(<McpAccessPageContent data={admin} />);

    await user.click(screen.getByRole('tab', { name: 'Tool access' }));
    expect(screen.getByText('1 unbound')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Grant' }));

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('/api/mcp-gateway/tools/tool/assignments'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({
            agent_id: 'agent-customer',
            user_ids: ['finance-user', 'intern'],
          }),
        }),
      ),
    );
  });
});
