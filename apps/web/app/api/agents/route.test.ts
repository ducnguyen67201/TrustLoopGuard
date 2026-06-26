import { beforeEach, describe, expect, it, vi } from 'vitest';

interface AgentProfileWire {
  agent_id: string;
  display_name: string;
  system_prompt?: string;
  workflow_definition?: {
    source: string;
    definition: Record<string, never>;
  };
  target_url?: string;
  scope: {
    in_scope: string[];
    out_of_scope: string[];
  };
  authority: {
    can_promise: string[];
    cannot_promise: string[];
  };
  tone: {
    target: string;
    forbidden: string[];
  };
  knowledge_sources: string[];
  escalation_triggers: string[];
}

interface PolicyDocumentWire {
  id: string;
}

interface AgentClient {
  upsertAgent: (profile: AgentProfileWire) => Promise<AgentProfileWire>;
  generateGuardrails: (agentId: string) => Promise<{ generated: PolicyDocumentWire[] }>;
  batchSetPolicyEnabled: (
    policyIds: string[],
    enabled: boolean,
  ) => Promise<{ policies: PolicyDocumentWire[] }>;
}

const mockState = vi.hoisted(() => {
  class MockRustApiError extends Error {
    readonly path = '';
    readonly status = 500;
    readonly body = '';
  }

  class MockWorkspaceAccessError extends Error {
    readonly status: 401 | 403 = 401;
  }

  return {
    upsertAgent: vi.fn<(profile: AgentProfileWire) => Promise<AgentProfileWire>>(),
    generateGuardrails: vi.fn<(agentId: string) => Promise<{ generated: PolicyDocumentWire[] }>>(),
    batchSetPolicyEnabled:
      vi.fn<
        (policyIds: string[], enabled: boolean) => Promise<{ policies: PolicyDocumentWire[] }>
      >(),
    rustApiForAuthorizedWorkspace: vi.fn<() => Promise<void>>(),
    tlClientForRequest: vi.fn<(req: Request) => Promise<AgentClient>>(),
    RustApiError: MockRustApiError,
    WorkspaceAccessError: MockWorkspaceAccessError,
  };
});

vi.mock('@/lib/server/tl-client', () => ({
  RustApiError: mockState.RustApiError,
  WorkspaceAccessError: mockState.WorkspaceAccessError,
  rustApiForAuthorizedWorkspace: mockState.rustApiForAuthorizedWorkspace,
  tlClientForRequest: mockState.tlClientForRequest,
}));

import { POST } from './route';

describe('/api/agents', () => {
  beforeEach(() => {
    mockState.upsertAgent.mockReset();
    mockState.generateGuardrails.mockReset();
    mockState.batchSetPolicyEnabled.mockReset();
    mockState.rustApiForAuthorizedWorkspace.mockReset();
    mockState.tlClientForRequest.mockReset();
    mockState.upsertAgent.mockImplementation(async (profile) => profile);
    mockState.generateGuardrails.mockResolvedValue({
      generated: [{ id: 'policy-1' }, { id: 'policy-2' }],
    });
    mockState.batchSetPolicyEnabled.mockResolvedValue({
      policies: [{ id: 'policy-1' }, { id: 'policy-2' }],
    });
    mockState.rustApiForAuthorizedWorkspace.mockResolvedValue();
    mockState.tlClientForRequest.mockResolvedValue({
      upsertAgent: mockState.upsertAgent,
      generateGuardrails: mockState.generateGuardrails,
      batchSetPolicyEnabled: mockState.batchSetPolicyEnabled,
    });
  });

  it('creates a prompt-backed agent and enables generated guardrails', async () => {
    const req = promptCreateRequest('https://app.test/api/agents?workspace=demo&environment=production');

    const res = await POST(req);
    const profile = mockState.upsertAgent.mock.calls[0]?.[0];

    expect(profile).toMatchObject({
      display_name: 'Support bot',
      system_prompt:
        'You are a customer support agent. Never promise refunds or legal outcomes.',
      target_url: 'http://127.0.0.1:9102',
    });
    expect(mockState.generateGuardrails).toHaveBeenCalledWith(profile?.agent_id);
    expect(mockState.batchSetPolicyEnabled).toHaveBeenCalledWith(['policy-1', 'policy-2'], true);
    expect(res.status).toBe(201);
    await expect(res.json()).resolves.toMatchObject({
      display_name: 'Support bot',
      generated_policy_count: 2,
      protected: true,
    });
  });

  it('creates a workflow-only agent without prompt guardrail generation', async () => {
    const req = new Request('https://app.test/api/agents?workspace=demo', {
      method: 'POST',
      body: JSON.stringify({
        displayName: 'Invoice workflow',
        workflowDefinition: { source: 'n8n', definition: { nodes: [] } },
        targetUrl: 'http://127.0.0.1:9102',
      }),
    });

    const res = await POST(req);

    expect(mockState.upsertAgent).toHaveBeenCalled();
    expect(mockState.generateGuardrails).not.toHaveBeenCalled();
    expect(mockState.batchSetPolicyEnabled).not.toHaveBeenCalled();
    expect(res.status).toBe(201);
    await expect(res.json()).resolves.toMatchObject({
      display_name: 'Invoice workflow',
      generated_policy_count: 0,
      protected: false,
    });
  });

  it('rejects invalid JSON before calling Rust', async () => {
    const res = await POST(
      new Request('https://app.test/api/agents', {
        method: 'POST',
        body: '{',
      }),
    );

    expect(res.status).toBe(400);
    expect(mockState.tlClientForRequest).not.toHaveBeenCalled();
    await expect(res.json()).resolves.toEqual({ error: 'invalid JSON body' });
  });

  it('cleans up and fails when prompt guardrail generation returns no policies', async () => {
    mockState.generateGuardrails.mockResolvedValue({ generated: [] });
    const req = promptCreateRequest();

    const res = await POST(req);
    const profile = mockState.upsertAgent.mock.calls[0]?.[0];

    expect(mockState.rustApiForAuthorizedWorkspace).toHaveBeenCalledWith(
      req,
      `/v1/agents/${encodeURIComponent(profile?.agent_id ?? '')}`,
      { method: 'DELETE' },
    );
    expect(mockState.batchSetPolicyEnabled).not.toHaveBeenCalled();
    expect(res.status).toBe(502);
    await expect(res.json()).resolves.toEqual({
      error: 'could not generate baseline protection policies',
    });
  });

  it('cleans up when enabling generated guardrails fails', async () => {
    mockState.batchSetPolicyEnabled.mockRejectedValue(new Error('enable failed'));
    const req = promptCreateRequest();

    const res = await POST(req);
    const profile = mockState.upsertAgent.mock.calls[0]?.[0];

    expect(mockState.rustApiForAuthorizedWorkspace).toHaveBeenCalledWith(
      req,
      `/v1/agents/${encodeURIComponent(profile?.agent_id ?? '')}`,
      { method: 'DELETE' },
    );
    expect(res.status).toBe(502);
    await expect(res.json()).resolves.toEqual({ error: 'upstream request failed' });
  });
});

function promptCreateRequest(url = 'https://app.test/api/agents?workspace=demo') {
  return new Request(url, {
    method: 'POST',
    body: JSON.stringify({
      displayName: 'Support bot',
      systemPrompt: 'You are a customer support agent. Never promise refunds or legal outcomes.',
      targetUrl: 'http://127.0.0.1:9102',
    }),
  });
}
