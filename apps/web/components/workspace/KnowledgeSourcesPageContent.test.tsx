import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/components/workspace/InviteMemberDialog', () => ({ InviteMemberDialog: () => null }));
vi.mock('@/components/workspace/DeleteWorkspaceDialog', () => ({
  DeleteWorkspaceDialog: () => null,
}));
vi.mock('@/components/workspace/KnowledgeSourceCreateDialog', () => ({
  KnowledgeSourceCreateDialog: () => <button type="button">Add source</button>,
}));
vi.mock('@/components/workspace/PendingInvitesTable', () => ({ PendingInvitesTable: () => null }));
vi.mock('@/components/workspace/QuickCreateAgentDialog', () => ({
  QuickCreateAgentDialog: () => null,
}));
vi.mock('@/components/workspace/RunDetailLiveView', () => ({ RunDetailLiveView: () => null }));
vi.mock('@/components/workspace/RunsLiveTable', () => ({ RunsLiveTable: () => null }));
vi.mock('@/components/analytics/AnalyticsChartGrid', () => ({ AnalyticsChartGrid: () => null }));
vi.mock('@/components/workspace/AgentEditDialog', () => ({ AgentEditDialog: () => null }));
vi.mock('@/components/workspace/ApprovalCheckerModeControl', () => ({
  ApprovalCheckerModeControl: () => null,
}));
vi.mock('@/components/workspace/GitHubIntegrationDialog', () => ({
  GitHubIntegrationDialog: () => null,
}));

import { KnowledgeSourcesPageContent } from './ManagementPages';

type PageData = Parameters<typeof KnowledgeSourcesPageContent>[0]['data'];

const data: PageData = {
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
  knowledgeSources: [],
};

afterEach(cleanup);

describe('KnowledgeSourcesPageContent', () => {
  it.each(['viewer', 'editor'])('hides create controls from %s members', (role) => {
    render(
      <KnowledgeSourcesPageContent
        data={{ ...data, activeWorkspace: { ...data.activeWorkspace, role } }}
      />,
    );

    expect(screen.queryByRole('button', { name: 'Add source' })).not.toBeInTheDocument();
  });

  it.each(['owner', 'admin'])('shows create controls to %s members', (role) => {
    render(
      <KnowledgeSourcesPageContent
        data={{ ...data, activeWorkspace: { ...data.activeWorkspace, role } }}
      />,
    );

    expect(screen.getAllByRole('button', { name: 'Add source' })).toHaveLength(2);
  });
});
