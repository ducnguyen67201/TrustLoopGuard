import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { AgentProfile, UpdateAgentInput } from '@/lib/agents';

import { AgentEditDialog } from './AgentEditDialog';

const mocks = vi.hoisted(() => ({
  getAgent: vi.fn<(agentId: string, signal?: AbortSignal) => Promise<AgentProfile>>(),
  updateAgent:
    vi.fn<
      (agentId: string, input: UpdateAgentInput, signal?: AbortSignal) => Promise<AgentProfile>
    >(),
  replace: vi.fn<(href: string) => void>(),
  refresh: vi.fn<() => void>(),
}));

vi.mock('next/navigation', () => ({
  usePathname: () => '/agents',
  useRouter: () => ({ replace: mocks.replace, refresh: mocks.refresh }),
  useSearchParams: () =>
    new URLSearchParams(
      'workspace=featherlane-ai-demo&environment=production&agent=agent-1&editAgent=agent-1',
    ),
}));

vi.mock('@/lib/agents', () => ({
  getAgent: mocks.getAgent,
  updateAgent: mocks.updateAgent,
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

const AGENT: AgentProfile = {
  agentId: 'agent-1',
  displayName: 'Test agent',
  systemPrompt:
    'You are a customer support agent. Escalate sensitive cases and never promise refunds.',
  workflowRequirements: [],
  scope: { inScope: ['customer support'], outOfScope: ['legal advice'] },
  authority: { canPromise: ['handoff'], cannotPromise: ['refunds'] },
  tone: { target: 'clear-professional', forbidden: ['dismissive'] },
  escalationTriggers: ['refund guarantee'],
};

describe('AgentEditDialog', () => {
  beforeEach(() => {
    mocks.getAgent.mockReset();
    mocks.updateAgent.mockReset();
    mocks.replace.mockReset();
    mocks.refresh.mockReset();
    mocks.getAgent.mockResolvedValue(AGENT);
  });

  afterEach(() => {
    cleanup();
  });

  it('opens the matching agent from the URL and preserves context when closed', async () => {
    const user = userEvent.setup();

    render(<AgentEditDialog agentId="agent-1" agentName="Test agent" />);

    expect(await screen.findByRole('heading', { name: 'Edit Test agent' })).toBeInTheDocument();
    await waitFor(() =>
      expect(mocks.getAgent).toHaveBeenCalledWith('agent-1', expect.any(AbortSignal)),
    );

    await user.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(mocks.replace).toHaveBeenCalledWith(
      '/agents?workspace=featherlane-ai-demo&environment=production&agent=agent-1',
    );
  });
});
