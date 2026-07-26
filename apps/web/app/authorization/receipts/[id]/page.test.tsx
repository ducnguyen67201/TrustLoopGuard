import { cleanup, render, screen } from '@testing-library/react';
import type { AuthorizationReceipt } from '@trustloopguard/sdk';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { DashboardShellData } from '@/lib/server/dashboard-data';

type GetDashboardShell = (
  workspaceSlug?: string | null,
  environmentId?: string | null,
) => Promise<DashboardShellData>;

type GetReceipt = (
  user: DashboardShellData['user'],
  workspaceId: string,
  path: string,
  init?: RequestInit,
  environmentId?: string | null,
) => Promise<AuthorizationReceipt>;

const mockState = vi.hoisted(() => {
  class MockRustApiError extends Error {
    constructor(
      public readonly path: string,
      public readonly status: number,
      public readonly body: string,
    ) {
      super(`Rust API ${path} failed with ${status}: ${body}`);
    }
  }

  return {
    getDashboardShell: vi.fn<GetDashboardShell>(),
    getReceipt: vi.fn<GetReceipt>(),
    notFound: vi.fn<() => never>(),
    RustApiError: MockRustApiError,
  };
});

vi.mock('next/navigation', () => ({
  notFound: mockState.notFound,
}));

vi.mock('@/lib/server/dashboard-data', () => ({
  getDashboardShell: mockState.getDashboardShell,
}));

vi.mock('@/lib/server/tl-client', () => ({
  RustApiError: mockState.RustApiError,
  rustApiForUserWorkspace: mockState.getReceipt,
}));

vi.mock('@/components/AppLayout', () => ({
  AppLayout: ({ children }: { children: ReactNode }) => <main>{children}</main>,
}));

vi.mock('@/components/workspace/AuthorizationReceiptContent', () => ({
  AuthorizationReceiptContent: ({ receipt }: { receipt: AuthorizationReceipt }) => (
    <div data-testid="receipt-content">{receipt.id}</div>
  ),
}));

import { RustApiError } from '@/lib/server/tl-client';
import AuthorizationReceiptPage from './page';

const activeWorkspace: DashboardShellData['activeWorkspace'] = {
  id: 'ws_nana',
  name: 'Nana',
  slug: 'nana',
  description: 'Customer workspace',
  policyCount: 1,
  enabledPolicies: 1,
  agentCount: 1,
  sourceCount: 0,
  apiKeyCount: 1,
  role: 'Owner',
  isKnowledgeBaseEnabled: false,
  isAttacksEnabled: false,
  isMcpGatewayEnabled: false,
};

const activeEnvironment: DashboardShellData['activeEnvironment'] = {
  id: 'production',
  slug: 'production',
  name: 'Production',
  description: null,
  isDefault: true,
};

const shell: DashboardShellData = {
  user: {
    id: '00000000-0000-4000-8000-000000000001',
    name: 'Nana Owner',
    email: 'owner@nana.test',
    avatar: '',
  },
  organization: {
    id: 'org_nana',
    name: 'Nana',
    slug: 'nana',
  },
  activeWorkspace,
  workspaces: [activeWorkspace],
  activeEnvironment,
  environments: [activeEnvironment],
  agents: [{ id: 'agent-1', name: 'Agent 1' }],
};

const receipt: AuthorizationReceipt = {
  id: 'receipt/one',
  trace_id: 'trace-1',
  principal_id: 'agent-1',
  operation: 'send_money',
  domain: 'tool',
  effect: 'permit',
  subject_hash: 'sha256:v1:subject',
  reason: 'current policy and authority permit the subject',
  findings: [],
  policy_versions: [],
  domain_evidence: { domain: 'tool', evidence: null },
  created_at: '2026-07-26T15:00:00Z',
};

const path = '/v1/authorization/receipts/receipt%2Fone';

function pageProps() {
  return {
    params: Promise.resolve({ id: 'receipt/one' }),
    searchParams: Promise.resolve({
      workspace: 'nana',
      environment: 'production',
    }),
  };
}

describe('AuthorizationReceiptPage', () => {
  beforeEach(() => {
    mockState.getDashboardShell.mockReset();
    mockState.getReceipt.mockReset();
    mockState.notFound.mockReset();
    mockState.getDashboardShell.mockResolvedValue(shell);
    mockState.getReceipt.mockResolvedValue(receipt);
    mockState.notFound.mockImplementation(() => {
      throw new Error('NEXT_HTTP_ERROR_FALLBACK;404');
    });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('loads the receipt with the resolved dashboard user and workspace context', async () => {
    render(await AuthorizationReceiptPage(pageProps()));

    expect(mockState.getDashboardShell).toHaveBeenCalledWith('nana', 'production');
    expect(mockState.getReceipt).toHaveBeenCalledWith(
      shell.user,
      'ws_nana',
      path,
      { method: 'GET' },
      'production',
    );
    expect(screen.getByTestId('receipt-content')).toHaveTextContent('receipt/one');
  });

  it('renders not found only when Rust reports a missing receipt', async () => {
    const error = new RustApiError(path, 404, 'receipt not found');
    mockState.getReceipt.mockRejectedValue(error);

    await expect(AuthorizationReceiptPage(pageProps())).rejects.toThrow(
      'NEXT_HTTP_ERROR_FALLBACK;404',
    );

    expect(mockState.notFound).toHaveBeenCalledOnce();
  });

  it('preserves authorization failures instead of masking them as not found', async () => {
    const error = new RustApiError(
      path,
      403,
      'signed-in user context is required to view authorization receipts',
    );
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    mockState.getReceipt.mockRejectedValue(error);

    await expect(AuthorizationReceiptPage(pageProps())).rejects.toBe(error);

    expect(mockState.notFound).not.toHaveBeenCalled();
    expect(consoleError).toHaveBeenCalledWith(
      '[authorization receipt] failed to load',
      path,
      error,
    );
  });
});
