import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import { WorkspaceDashboard, type DashboardUsageData } from './WorkspaceDashboard';
import type { WorkspaceDashboardData } from '@/lib/server/dashboard-data';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ refresh: vi.fn(), push: vi.fn() }),
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

beforeAll(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  Object.defineProperty(HTMLElement.prototype, 'hasPointerCapture', {
    configurable: true,
    value: vi.fn(() => false),
  });
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  });
});

const WORKSPACE_SLUG = 'demo';
const STORAGE_KEY = `tlg:dashboard-layout:${WORKSPACE_SLUG}`;

function makeData(): WorkspaceDashboardData {
  const workspace = {
    id: 'ws_1',
    name: 'Demo Workspace',
    slug: WORKSPACE_SLUG,
    description: '',
    policyCount: 3,
    enabledPolicies: 2,
    agentCount: 1,
    sourceCount: 0,
    apiKeyCount: 1,
    role: 'owner',
    isKnowledgeBaseEnabled: false,
    isAttacksEnabled: false,
  };
  const environment = {
    id: 'env_prod',
    slug: 'production',
    name: 'Production',
    description: null,
    isDefault: true,
  };
  return {
    user: { id: 'user_1', name: 'Ada', email: 'ada@example.com', avatar: '' },
    organization: { id: 'org_1', name: 'Acme', slug: 'acme' },
    activeWorkspace: workspace,
    workspaces: [workspace],
    activeEnvironment: environment,
    environments: [environment],
    agents: [{ id: 'agent_1', name: 'Support bot' }],
    metrics: [{ label: 'Requests', value: '120', delta: '+4%', detail: 'last 24h' }],
    recentDecisions: [],
    settings: {
      defaultAction: 'permit',
      escalationWebhookUrl: null,
      telemetryEnabled: true,
      retentionDays: '30',
    },
  };
}

const emptyUsage: DashboardUsageData = {
  dayBuckets: [],
  principalBuckets: [],
  modelBuckets: [],
};

function renderDashboard() {
  return render(<WorkspaceDashboard data={makeData()} usage={emptyUsage} usagePeriod="week" />);
}

function createStorageMock(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: vi.fn(() => values.clear()),
    getItem: vi.fn((key: string) => values.get(key) ?? null),
    key: vi.fn((index: number) => Array.from(values.keys())[index] ?? null),
    removeItem: vi.fn((key: string) => {
      values.delete(key);
    }),
    setItem: vi.fn((key: string, value: string) => {
      values.set(key, value);
    }),
  };
}

describe('WorkspaceDashboard customization', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: createStorageMock(),
    });
  });

  afterEach(() => {
    cleanup();
    localStorage.clear();
    vi.stubGlobal('fetch', undefined);
  });

  it('renders the default widgets', () => {
    renderDashboard();
    expect(screen.getByText('Recent decisions')).toBeInTheDocument();
    expect(screen.getByText('How the guardrail behaves')).toBeInTheDocument();
    expect(screen.getByText('Set up your protection')).toBeInTheDocument();
  });

  it('hides a widget when unchecked and persists the choice', async () => {
    const user = userEvent.setup();
    renderDashboard();

    expect(screen.getByText('How the guardrail behaves')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /customize/i }));
    await user.click(
      await screen.findByRole('menuitemcheckbox', { name: 'How the guardrail behaves' }),
    );
    await user.keyboard('{Escape}');

    expect(screen.queryByText('How the guardrail behaves')).not.toBeInTheDocument();

    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}');
    expect(stored.hidden).toContain('guardrail-config');
  });

  it('restores hidden widgets on reset to default', async () => {
    const user = userEvent.setup();
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        order: [
          'recent-decisions',
          'decision-mix',
          'guardrail-config',
          'usage',
          'metrics',
          'setup-shortcuts',
        ],
        hidden: ['guardrail-config'],
      }),
    );
    renderDashboard();

    expect(screen.queryByText('How the guardrail behaves')).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /customize/i }));
    await user.click(await screen.findByRole('menuitem', { name: /reset to default/i }));

    expect(screen.getByText('How the guardrail behaves')).toBeInTheDocument();
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });
});
