import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { PoliciesPageContent } from './PoliciesPageContent';
import type { DashboardShellData, FamilyPolicyRow } from '@/lib/server/dashboard-data';

const getPolicy = vi.fn();

vi.mock('next/navigation', () => ({
  useRouter: () => ({ refresh: vi.fn() }),
}));

vi.mock('@/lib/policies', () => ({
  aiEditPolicy: vi.fn(),
  deletePolicy: vi.fn(),
  getPolicy: (id: string) => getPolicy(id),
  getPolicyVersion: vi.fn(),
  listPolicyVersions: vi.fn(),
  setPoliciesEnabled: vi.fn(),
  setPolicyEnabled: vi.fn(),
  upsertPolicy: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

describe('PoliciesPageContent', () => {
  it('opens the financial editor for financial registry rows', async () => {
    render(<PoliciesPageContent data={pageData([financialPolicy()])} />);

    await userEvent.click(screen.getByRole('button', { name: /actions/i }));
    await userEvent.click(await screen.findByRole('menuitem', { name: /edit policy/i }));

    expect(screen.getByRole('heading', { name: /edit financial policy/i })).toBeInTheDocument();
    expect(screen.getByDisplayValue('refund-bot-refund-controls')).toBeDisabled();
    expect(screen.getByDisplayValue('500')).toBeInTheDocument();
    expect(getPolicy).not.toHaveBeenCalled();
  });
});

type PoliciesPageDataForTest = Parameters<typeof PoliciesPageContent>[0]['data'];

function pageData(familyPolicies: FamilyPolicyRow[]): PoliciesPageDataForTest {
  return {
    ...shellData(),
    agents: [],
    policies: [],
    familyPolicies,
  };
}

function shellData(): DashboardShellData {
  return {
    user: {
      id: 'user_1',
      name: 'Duc',
      email: 'duc@example.com',
      avatar: 'D',
    },
    organization: {
      id: 'org_1',
      name: 'Test',
      slug: 'test',
    },
    activeWorkspace: {
      id: 'ws_1',
      name: 'Test',
      slug: 'test',
      description: '',
      policyCount: 1,
      enabledPolicies: 1,
      agentCount: 0,
      sourceCount: 0,
      apiKeyCount: 0,
      role: 'owner',
      isKnowledgeBaseEnabled: false,
      isAttacksEnabled: false,
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
  };
}

function financialPolicy(): FamilyPolicyRow {
  return {
    id: 'refund-bot-refund-controls',
    description: 'Refund controls for support agents',
    severity: 'high',
    when: {
      agents: ['refund-bot'],
      action_kinds: ['refund'],
      operations: ['issue_refund'],
      currencies: ['USD'],
      rails: ['payment_http'],
    },
    per_transaction_minor: 10_000,
    approval_threshold_minor: 5_000,
    daily_minor: 50_000,
    monthly_minor: 500_000,
    required_preconditions: ['order_exists', 'amount_lte_refundable_balance'],
    missing_evidence_effect: 'require_approval',
    failed_precondition_effect: 'deny',
    on_breach: 'deny',
    enabled: true,
  };
}
