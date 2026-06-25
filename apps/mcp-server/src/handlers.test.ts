import { describe, expect, it, vi } from 'vitest';

import { SdkError, Unauthorized, type Decision, type GuardEvent } from '@trustloopguard/sdk';

import { createToolHandlers, type TrustLoopClient } from './handlers';

function decision(): Decision {
  return {
    trace_id: 'trace_1',
    verdict: 'allow',
    reason: 'ok',
    triggered_policies: [],
    safe_output: null,
    latency_ms: 1,
    tier_results: [],
  };
}

function event(): GuardEvent {
  return {
    kind: 'tool.call.proposed',
    principal: {
      workspace_id: '',
      environment_id: '',
      agent_id: 'agent-1',
    },
    action: {
      operation: 'send_email',
      parameters: { to: 'a@example.com' },
    },
    sources: [],
    provenance: {},
    context: null,
  };
}

function client(overrides: Partial<TrustLoopClient> = {}): TrustLoopClient {
  return {
    submitEvent: vi.fn<TrustLoopClient['submitEvent']>(async () => decision()),
    startRun: vi.fn<TrustLoopClient['startRun']>(async () => ({ id: 'run_1' })),
    listRuns: vi.fn<TrustLoopClient['listRuns']>(async () => ({ runs: [] })),
    getRun: vi.fn<TrustLoopClient['getRun']>(async () => ({ run: { id: 'run_1' } })),
    createRunEvent: vi.fn<TrustLoopClient['createRunEvent']>(async () => ({ id: 'event_1' })),
    finishRun: vi.fn<TrustLoopClient['finishRun']>(async () => ({ id: 'run_1', status: 'completed' })),
    validatePolicy: vi.fn<TrustLoopClient['validatePolicy']>(async () => ({ valid: true, errors: [] })),
    listPolicies: vi.fn<TrustLoopClient['listPolicies']>(async () => ({ policies: [] })),
    getPolicy: vi.fn<TrustLoopClient['getPolicy']>(async () => ({ id: 'p1' })),
    upsertPolicy: vi.fn<TrustLoopClient['upsertPolicy']>(async () => ({ id: 'p1' })),
    setPolicyEnabled: vi.fn<TrustLoopClient['setPolicyEnabled']>(async () => ({
      id: 'p1',
      enabled: true,
    })),
    listAgents: vi.fn<TrustLoopClient['listAgents']>(async () => ({ agents: [] })),
    upsertAgent: vi.fn<TrustLoopClient['upsertAgent']>(async () => ({ agent_id: 'agent-1' })),
    listToolMetadata: vi.fn<TrustLoopClient['listToolMetadata']>(async () => ({ tools: [] })),
    upsertToolMetadata: vi.fn<TrustLoopClient['upsertToolMetadata']>(async () => ({
      metadata: { tool: 'send_email' },
      enabled: true,
    })),
    listTraces: vi.fn<TrustLoopClient['listTraces']>(async () => ({ traces: [] })),
    listRunTraces: vi.fn<TrustLoopClient['listRunTraces']>(async () => ({ traces: [] })),
    ...overrides,
  };
}

describe('createToolHandlers', () => {
  it('submits guard events and returns JSON text', async () => {
    const fakeClient = client();
    const result = await createToolHandlers(fakeClient).submit_guard_event({ event: event() });

    expect(fakeClient.submitEvent).toHaveBeenCalledWith(event());
    expect(result.isError).toBeUndefined();
    expect(JSON.parse(result.content[0]!.text)).toMatchObject({ verdict: 'allow' });
  });

  it('finishes runs as completed by default', async () => {
    const fakeClient = client();
    await createToolHandlers(fakeClient).finish_run({ run_id: 'run_1' });

    expect(fakeClient.finishRun).toHaveBeenCalledWith('run_1', 'completed');
  });

  it('exposes setup and inspection tools through the SDK client', async () => {
    const fakeClient = client();
    const handlers = createToolHandlers(fakeClient);

    await handlers.upsert_agent({ agent_id: 'agent-1', display_name: 'Agent' });
    await handlers.list_runs({});
    await handlers.get_run({ run_id: 'run_1' });
    await handlers.upsert_policy({ source: 'id: p1' });
    await handlers.set_policy_enabled({ policy_id: 'p1', enabled: true });
    await handlers.upsert_tool_metadata({ tool: 'send_email', side_effect: 'external_communication' });

    expect(fakeClient.upsertAgent).toHaveBeenCalledOnce();
    expect(fakeClient.listRuns).toHaveBeenCalledOnce();
    expect(fakeClient.getRun).toHaveBeenCalledWith('run_1');
    expect(fakeClient.upsertPolicy).toHaveBeenCalledWith('id: p1');
    expect(fakeClient.setPolicyEnabled).toHaveBeenCalledWith('p1', true);
    expect(fakeClient.upsertToolMetadata).toHaveBeenCalledOnce();
  });

  it('maps SDK errors to MCP tool errors without stack traces', async () => {
    const error: SdkError = new Unauthorized({
      code: 'unauthorized',
      message: 'invalid bearer token',
      retriable: false,
      details: null,
    });
    const fakeClient = client({
      submitEvent: vi.fn<TrustLoopClient['submitEvent']>(async () => {
        throw error;
      }),
    });

    const result = await createToolHandlers(fakeClient).submit_guard_event({ event: event() });

    expect(result.isError).toBe(true);
    expect(result.content[0]!.text).toContain('unauthorized: invalid bearer token');
    expect(result.content[0]!.text).not.toContain('at ');
  });
});
