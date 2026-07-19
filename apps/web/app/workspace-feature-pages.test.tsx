import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  getDashboardShell: vi.fn(),
  getKnowledgePageData: vi.fn(),
  getMcpAccessPageData: vi.fn(),
  notFound: vi.fn(() => {
    throw new Error('NOT_FOUND');
  }),
}));

vi.mock('next/navigation', () => ({ notFound: mocks.notFound }));
vi.mock('@/lib/server/dashboard-data', () => ({
  getDashboardShell: mocks.getDashboardShell,
  getKnowledgePageData: mocks.getKnowledgePageData,
  getMcpAccessPageData: mocks.getMcpAccessPageData,
}));
vi.mock('@/components/AppLayout', () => ({
  AppLayout: ({ children }: { children: React.ReactNode }) => children,
}));
vi.mock('@/components/workspace/ManagementPages', () => ({
  KnowledgeSourcesPageContent: () => null,
}));
vi.mock('@/components/workspace/KnowledgeSourceForm', () => ({
  KnowledgeSourceForm: () => null,
}));
vi.mock('@/components/workspace/McpAccessPageContent', () => ({ McpAccessPageContent: () => null }));
vi.mock('./attacks/_components/attacks-panel', () => ({ AttacksPanel: () => null }));

import AttacksPage from './attacks/page';
import NewKnowledgeSourcePage from './knowledge-sources/new/page';
import KnowledgeSourcesPage from './knowledge-sources/page';
import McpAccessPage from './mcp-access/page';

const disabledShell = {
  activeWorkspace: {
    isAttacksEnabled: false,
    isKnowledgeBaseEnabled: false,
    isMcpGatewayEnabled: false,
  },
};

describe('workspace feature pages', () => {
  beforeEach(() => {
    mocks.getDashboardShell.mockReset();
    mocks.getKnowledgePageData.mockReset();
    mocks.getMcpAccessPageData.mockReset();
    mocks.notFound.mockClear();
  });

  it('returns not found for a disabled attacks workspace', async () => {
    mocks.getDashboardShell.mockResolvedValue(disabledShell);

    await expect(
      AttacksPage({ searchParams: Promise.resolve({ workspace: 'acme' }) }),
    ).rejects.toThrow('NOT_FOUND');
  });

  it('returns not found for a disabled knowledge-base workspace', async () => {
    mocks.getKnowledgePageData.mockResolvedValue(disabledShell);

    await expect(
      KnowledgeSourcesPage({ searchParams: Promise.resolve({ workspace: 'acme' }) }),
    ).rejects.toThrow('NOT_FOUND');
  });

  it('renders attacks for an enabled workspace', async () => {
    mocks.getDashboardShell.mockResolvedValue({
      activeWorkspace: { ...disabledShell.activeWorkspace, isAttacksEnabled: true },
    });

    await expect(
      AttacksPage({ searchParams: Promise.resolve({ workspace: 'acme' }) }),
    ).resolves.toBeDefined();
  });

  it('renders knowledge sources for an enabled workspace', async () => {
    mocks.getKnowledgePageData.mockResolvedValue({
      activeWorkspace: {
        ...disabledShell.activeWorkspace,
        isKnowledgeBaseEnabled: true,
      },
    });

    await expect(
      KnowledgeSourcesPage({ searchParams: Promise.resolve({ workspace: 'acme' }) }),
    ).resolves.toBeDefined();
  });

  it('returns not found for the disabled new knowledge-source page', async () => {
    mocks.getDashboardShell.mockResolvedValue(disabledShell);

    await expect(
      NewKnowledgeSourcePage({ searchParams: Promise.resolve({ workspace: 'acme' }) }),
    ).rejects.toThrow('NOT_FOUND');
  });

  it('returns not found for disabled MCP access', async () => {
    mocks.getMcpAccessPageData.mockResolvedValue(disabledShell);
    await expect(McpAccessPage({ searchParams: Promise.resolve({ workspace: 'acme' }) })).rejects.toThrow('NOT_FOUND');
  });

  it('renders MCP access when enabled', async () => {
    mocks.getMcpAccessPageData.mockResolvedValue({ activeWorkspace: { ...disabledShell.activeWorkspace, isMcpGatewayEnabled: true } });
    await expect(McpAccessPage({ searchParams: Promise.resolve({ workspace: 'acme' }) })).resolves.toBeDefined();
  });
});
