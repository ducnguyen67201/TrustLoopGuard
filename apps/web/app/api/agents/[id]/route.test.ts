import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/server/tl-client', () => {
  class MockRustApiError extends Error {}
  class MockWorkspaceAccessError extends Error {}
  return {
    RustApiError: MockRustApiError,
    WorkspaceAccessError: MockWorkspaceAccessError,
    rustApiForAuthorizedWorkspace: vi.fn(),
  };
});

import { rustApiForAuthorizedWorkspace } from '@/lib/server/tl-client';
import { GET, PUT } from './route';

const rustMock = vi.mocked(rustApiForAuthorizedWorkspace);

const AGENT = {
  agent_id: 'agent-1',
  display_name: 'NorthPay Disputes',
  scope: { in_scope: ['payment disputes'], out_of_scope: ['legal advice'] },
  authority: {
    can_promise: ['request verification'],
    cannot_promise: ['refunds to arbitrary accounts'],
  },
  tone: { target: 'clear-professional', forbidden: ['dismissive'] },
  system_prompt:
    'You are NorthPay Disputes. Help customers with payment disputes and escalate risky refunds.',
  target_url: 'http://127.0.0.1:9202',
  knowledge_sources: [],
  escalation_triggers: ['refund destination changed'],
};

function context(id = 'agent-1') {
  return { params: Promise.resolve({ id }) };
}

function putRequest(body: unknown): Request {
  return new Request('https://app.test/api/agents/agent-1', {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
}

describe('/api/agents/[id]', () => {
  beforeEach(() => {
    rustMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('fetches one agent from Rust', async () => {
    rustMock.mockResolvedValue(AGENT);

    const res = await GET(new Request('https://app.test/api/agents/agent-1'), context());

    expect(res.status).toBe(200);
    expect(rustMock).toHaveBeenCalledWith(
      expect.any(Request),
      '/v1/agents/agent-1',
    );
  });

  it('updates an existing agent without changing its id', async () => {
    rustMock.mockResolvedValue({ ...AGENT, display_name: 'NorthPay Guarded' });

    const res = await PUT(
      putRequest({
        displayName: 'NorthPay Guarded',
        systemPrompt:
          'You are NorthPay Guarded. Help customers with payment disputes and escalate risky refunds.',
        targetUrl: 'http://127.0.0.1:9202',
        scope: { inScope: ['payment disputes'], outOfScope: ['legal advice'] },
        authority: {
          canPromise: ['request verification'],
          cannotPromise: ['refunds to arbitrary accounts'],
        },
        tone: { target: 'clear-professional', forbidden: ['dismissive'] },
        escalationTriggers: ['refund destination changed'],
      }),
      context(),
    );

    expect(res.status).toBe(200);
    const [, path, init] = rustMock.mock.calls[0] ?? [];
    expect(path).toBe('/v1/agents');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(String(init?.body))).toMatchObject({
      agent_id: 'agent-1',
      display_name: 'NorthPay Guarded',
      target_url: 'http://127.0.0.1:9202',
      scope: {
        in_scope: ['payment disputes'],
        out_of_scope: ['legal advice'],
      },
      authority: {
        can_promise: ['request verification'],
        cannot_promise: ['refunds to arbitrary accounts'],
      },
      tone: {
        target: 'clear-professional',
        forbidden: ['dismissive'],
      },
      escalation_triggers: ['refund destination changed'],
    });
  });

  it('rejects non-loopback target URLs', async () => {
    const res = await PUT(
      putRequest({
        displayName: 'Bad target',
        systemPrompt:
          'You are a customer support agent with enough prompt text to pass validation.',
        targetUrl: 'https://example.com/agent',
        scope: { inScope: [], outOfScope: [] },
        authority: { canPromise: [], cannotPromise: [] },
        tone: { target: 'clear-professional', forbidden: [] },
        escalationTriggers: [],
      }),
      context(),
    );

    expect(res.status).toBe(400);
    expect(rustMock).not.toHaveBeenCalled();
  });
});
