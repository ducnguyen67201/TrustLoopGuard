import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  getDashboardShell: vi.fn(),
  getKnowledgePageData: vi.fn(),
  notFound: vi.fn(() => {
    throw new Error('NOT_FOUND');
  }),
}));

vi.mock('next/navigation', () => ({ notFound: mocks.notFound }));
vi.mock('@/lib/server/dashboard-data', () => ({
  getDashboardShell: mocks.getDashboardShell,
  getKnowledgePageData: mocks.getKnowledgePageData,
}));
vi.mock('@/components/AppLayout', () => ({
  AppLayout: ({ children }: { children: React.ReactNode }) => children,
}));
vi.mock('@/components/workspace/ManagementPages', () => ({
  KnowledgeSourcesPageContent: () => null,
}));
vi.mock('./attacks/_components/attacks-panel', () => ({ AttacksPanel: () => null }));

import AttacksPage from './attacks/page';
import KnowledgeSourcesPage from './knowledge-sources/page';

const disabledShell = {
  activeWorkspace: {
    isAttacksEnabled: false,
    isKnowledgeBaseEnabled: false,
  },
};

describe('workspace feature pages', () => {
  beforeEach(() => {
    mocks.getDashboardShell.mockReset();
    mocks.getKnowledgePageData.mockReset();
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
});
