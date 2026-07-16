import { describe, expect, it, vi } from 'vitest';

import {
  Client,
  GuardedToolBlocked,
  ToolRegistrationMode,
  guardAgent,
  type GuardToolDiscoveryWarning,
} from '../src';
import { toolSchemaHash } from '../src/tool-discovery';
import { mockFetch } from './test-utils';

interface ToolWireEvent {
  kind: string;
  principal: {
    agent_id: string;
  };
  action: {
    operation: string;
    parameters: Record<string, ToolValue>;
    tool_identity: {
      server_id: string;
      tool_name: string;
      schema_hash: string;
    };
    authorization?: {
      grant_id: string;
      attempt_id: string;
    };
  };
}

type ToolValue = object | string | number | boolean | null;

function decision(effect: 'permit' | 'deny' | 'require_approval', extra: object = {}): object {
  return {
    trace_id: `trace-${effect}`,
    domain: 'tool',
    effect,
    reason: effect,
    findings: [],
    transformed_value: null,
    latency_ms: 1,
    ...extra,
  };
}

function permitClient(): {
  client: Client;
  fetchSpy: ReturnType<typeof mockFetch>;
} {
  const fetchSpy = mockFetch(async () => Response.json(decision('permit')));
  return {
    client: new Client({ baseUrl: 'http://x', fetchImpl: fetchSpy }),
    fetchSpy,
  };
}

function requestUrl(input: RequestInfo | URL): string {
  return typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
}

function requestBody(init: RequestInit | undefined): ToolWireEvent {
  if (init?.body === undefined || init.body === null) {
    throw new Error('expected request body');
  }
  return JSON.parse(String(init.body)) as ToolWireEvent;
}

describe('guardAgent() tool discovery', () => {
  it('discovers and guards a Mastra-shaped object-map tool', async () => {
    const { client, fetchSpy } = permitClient();
    const runtime = { requestId: 'req-1' };
    const execute = vi.fn(
      async (input: { location: string }, context: { requestId: string }) =>
        `${input.location}:${context.requestId}`,
    );
    const agent = guardAgent(
      {
        tools: {
          weatherTool: {
            id: 'get-weather',
            description: 'Get current weather for a location',
            inputSchema: {
              type: 'object',
              properties: { location: { type: 'string' } },
            },
            outputSchema: {
              type: 'object',
              properties: { output: { type: 'string' } },
            },
            execute,
          },
        },
        async reply(message: string): Promise<string> {
          return message;
        },
      },
      { agentId: 'weather-agent', client },
    );

    const result = await agent.tools.weatherTool.execute({ location: 'SF' }, runtime);

    expect(result).toBe('SF:req-1');
    expect(execute).toHaveBeenCalledOnce();
    expect(execute.mock.calls[0]?.[1]).toBe(runtime);
    expect(fetchSpy).toHaveBeenCalledOnce();
    const body = requestBody(fetchSpy.mock.calls[0]?.[1]);
    expect(body.kind).toBe('tool.call.proposed');
    expect(body.principal.agent_id).toBe('weather-agent');
    expect(body.action.operation).toBe('get-weather');
    expect(body.action.parameters).toEqual({ location: 'SF' });
    expect(body.action.tool_identity).toMatchObject({
      server_id: 'mastra',
      tool_name: 'get-weather',
    });
    expect(body.action.tool_identity.schema_hash).toMatch(/^tlg-schema:fnv1a64:/);
  });

  it('wraps tools returned by Mastra getToolsForExecution()', async () => {
    const { client, fetchSpy } = permitClient();
    const execute = vi.fn(async (input: { appointmentId: string }) => input.appointmentId);
    const bookingTool = {
      id: 'book-appointment',
      description: 'Book one appointment',
      inputSchema: { type: 'object', properties: { appointmentId: { type: 'string' } } },
      execute,
    };

    class MastraLikeAgent {
      async getToolsForExecution(): Promise<{ bookingTool: typeof bookingTool }> {
        return { bookingTool };
      }

      async runTool(appointmentId: string): Promise<string> {
        const tools = await this.getToolsForExecution();
        return await tools.bookingTool.execute({ appointmentId });
      }
    }

    const agent = guardAgent(new MastraLikeAgent(), {
      agentId: 'booking-agent',
      client,
    });

    await expect(agent.runTool('appt-1')).resolves.toBe('appt-1');
    expect(execute).toHaveBeenCalledOnce();
    expect(fetchSpy).toHaveBeenCalledOnce();
    expect(requestBody(fetchSpy.mock.calls[0]?.[1]).action.operation).toBe('book-appointment');
  });

  it('discovers OpenAI Agents JS function tools from agent.tools', async () => {
    const { client, fetchSpy } = permitClient();
    const execute = vi.fn(async (input: { city: string }) => `sunny in ${input.city}`);
    const agent = guardAgent(
      {
        tools: [
          {
            type: 'function',
            name: 'get_weather',
            description: 'Get the weather for a city',
            parameters: {
              type: 'object',
              properties: { city: { type: 'string' } },
            },
            execute,
          },
        ],
      },
      { agentId: 'openai-agent', client },
    );

    await expect(agent.tools[0]!.execute({ city: 'Seattle' })).resolves.toBe('sunny in Seattle');
    const body = requestBody(fetchSpy.mock.calls[0]?.[1]);
    expect(body.action.tool_identity.server_id).toBe('openai-agents');
    expect(body.action.parameters).toEqual({ city: 'Seattle' });
  });

  it('uses LiveKit toolCtx.updateTools() when a frozen tool must be replaced', async () => {
    const { client, fetchSpy } = permitClient();
    const execute = vi.fn(async (input: { orderId: string }) => input.orderId);
    const frozenTool = Object.freeze({
      id: 'getOrderStatus',
      name: 'getOrderStatus',
      description: 'Look up an order',
      parameters: {
        type: 'object',
        properties: { orderId: { type: 'string' } },
      },
      flags: 0,
      execute,
    });

    class ToolContext {
      private entries: object[] = [frozenTool];
      readonly updateTools = vi.fn((tools: readonly object[]) => {
        this.entries = [...tools];
      });

      get tools(): readonly object[] {
        return [...this.entries];
      }
    }

    const toolCtx = new ToolContext();
    const agent = guardAgent({ toolCtx }, { agentId: 'voice-agent', client });
    const guardedTool = toolCtx.tools[0] as {
      execute(input: { orderId: string }): Promise<string>;
    };

    await expect(guardedTool.execute({ orderId: 'order-1' })).resolves.toBe('order-1');
    expect(toolCtx.updateTools).toHaveBeenCalledOnce();
    expect(execute).toHaveBeenCalledOnce();
    expect(requestBody(fetchSpy.mock.calls[0]?.[1]).action.tool_identity.server_id).toBe('livekit');
  });

  it('blocks denied tool calls before the original side effect runs', async () => {
    const fetchImpl = mockFetch(async () => Response.json(decision('deny')));
    const client = new Client({ baseUrl: 'http://x', fetchImpl });
    const execute = vi.fn(async () => 'sent');
    const agent = guardAgent(
      {
        tools: [
          {
            type: 'function',
            name: 'send_email',
            description: 'Send an email',
            parameters: { type: 'object' },
            execute,
          },
        ],
      },
      { agentId: 'mail-agent', client },
    );

    await expect(agent.tools[0]!.execute({ to: 'customer@example.com' })).rejects.toBeInstanceOf(
      GuardedToolBlocked,
    );
    expect(execute).not.toHaveBeenCalled();
  });

  it('resumes approval and executes the wrapped tool exactly once', async () => {
    const approvalId = '018f1111-1111-7111-8111-111111111111';
    const grantId = '018f2222-2222-7222-8222-222222222222';
    const leaseId = '018f3333-3333-7333-8333-333333333333';
    let eventCount = 0;
    const fetchImpl = mockFetch(async (input) => {
      const url = requestUrl(input);
      if (url.includes(`/authorization/approvals/${approvalId}`)) {
        return Response.json({ id: approvalId, status: 'approved', grant_id: grantId });
      }
      if (url.includes(`/authorization/leases/${leaseId}/complete`)) {
        return Response.json({ id: leaseId, status: 'consumed' });
      }
      eventCount += 1;
      if (eventCount === 1) {
        return Response.json(
          decision('require_approval', {
            approval: {
              id: approvalId,
              status: 'pending',
              envelope_hash: 'sha256:v1:reviewed',
              expires_at: '2026-07-17T00:00:00Z',
              poll_after_ms: 1,
            },
          }),
        );
      }
      return Response.json(
        decision('permit', {
          lease: {
            id: leaseId,
            intent_id: '018f4444-4444-7444-8444-444444444444',
            grant_id: grantId,
            attempt_id: 'attempt-1',
            fingerprint: 'sha256:v1:subject',
            status: 'claimed',
            claimed_at: '2026-07-16T00:00:00Z',
            expires_at: '2026-07-16T00:05:00Z',
          },
        }),
      );
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl });
    const execute = vi.fn(async (input: { appointmentId: string }) => input.appointmentId);
    const agent = guardAgent(
      {
        tools: {
          bookAppointment: {
            id: 'book-appointment',
            description: 'Book an appointment',
            inputSchema: { type: 'object' },
            execute,
          },
        },
      },
      { agentId: 'booking-agent', client },
    );

    await expect(agent.tools.bookAppointment.execute({ appointmentId: 'appt-1' })).resolves.toBe(
      'appt-1',
    );
    expect(execute).toHaveBeenCalledOnce();
    expect(eventCount).toBe(2);
  });

  it('registers metadata lazily once before authorizing the first tool call', async () => {
    const endpoints: string[] = [];
    const bodies: object[] = [];
    const fetchImpl = mockFetch(async (input, init) => {
      const url = requestUrl(input);
      endpoints.push(url);
      if (init?.body !== undefined && init.body !== null) {
        bodies.push(JSON.parse(String(init.body)) as object);
      }
      if (url.endsWith('/v1/tool-metadata')) {
        const metadata = bodies.at(-1);
        return Response.json({ metadata, enabled: true }, { status: 201 });
      }
      return Response.json(decision('permit'));
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl });
    const execute = vi.fn(async (input: { to: string }) => input.to);
    const agent = guardAgent(
      {
        tools: {
          sendEmail: {
            id: 'send-email',
            description: 'Send an email',
            inputSchema: { type: 'object' },
            execute,
          },
        },
      },
      {
        agentId: 'mail-agent',
        client,
        tools: {
          register: ToolRegistrationMode.Strict,
          inferMetadata: () => ({
            side_effect: 'external_communication',
            reversible: false,
            params: [],
          }),
        },
      },
    );

    await agent.tools.sendEmail.execute({ to: 'a@example.com' });
    await agent.tools.sendEmail.execute({ to: 'b@example.com' });

    expect(endpoints.filter((url) => url.endsWith('/v1/tool-metadata'))).toHaveLength(1);
    expect(endpoints.filter((url) => url.endsWith('/v1/events'))).toHaveLength(2);
    expect(endpoints[0]).toBe('http://x/v1/tool-metadata');
    expect(bodies[0]).toMatchObject({
      tool: 'send-email',
      side_effect: 'external_communication',
      reversible: false,
      enabled: true,
    });
  });

  it('fails before authorization when strict metadata registration fails', async () => {
    const fetchImpl = mockFetch(async () => {
      return Response.json(
        {
          code: 'internal',
          message: 'registration unavailable',
          retriable: false,
          details: null,
        },
        { status: 500 },
      );
    });
    const client = new Client({
      baseUrl: 'http://x',
      fetchImpl,
      retry: { maxAttempts: 1, baseS: 0, capS: 0, totalBudgetS: 0 },
    });
    const execute = vi.fn(async () => 'done');
    const agent = guardAgent(
      {
        tools: {
          mutate: {
            id: 'mutate',
            description: 'Mutate state',
            inputSchema: { type: 'object' },
            execute,
          },
        },
      },
      {
        agentId: 'strict-agent',
        client,
        tools: { register: ToolRegistrationMode.Strict },
      },
    );

    await expect(agent.tools.mutate.execute({ id: '1' })).rejects.toThrow(
      'registration unavailable',
    );
    expect(execute).not.toHaveBeenCalled();
    expect(fetchImpl).toHaveBeenCalledOnce();
  });

  it('continues authorization after best-effort registration failure and reports a warning', async () => {
    const warnings: GuardToolDiscoveryWarning[] = [];
    let callCount = 0;
    const fetchImpl = mockFetch(async () => {
      callCount += 1;
      if (callCount === 1) {
        return Response.json(
          {
            code: 'internal',
            message: 'registration unavailable',
            retriable: false,
            details: null,
          },
          { status: 500 },
        );
      }
      return Response.json(decision('permit'));
    });
    const client = new Client({
      baseUrl: 'http://x',
      fetchImpl,
      retry: { maxAttempts: 1, baseS: 0, capS: 0, totalBudgetS: 0 },
    });
    const execute = vi.fn(async () => 'done');
    const agent = guardAgent(
      {
        tools: {
          mutate: {
            id: 'mutate',
            description: 'Mutate state',
            inputSchema: { type: 'object' },
            execute,
          },
        },
      },
      {
        agentId: 'best-effort-agent',
        client,
        tools: {
          register: ToolRegistrationMode.BestEffort,
          onDiscoveryWarning: (warning) => warnings.push(warning),
        },
      },
    );

    await expect(agent.tools.mutate.execute({ id: '1' })).resolves.toBe('done');
    expect(execute).toHaveBeenCalledOnce();
    expect(warnings).toHaveLength(1);
    expect(warnings[0]?.code).toBe('registration_failed');
  });

  it('does not register metadata when registration is off', async () => {
    const { client, fetchSpy } = permitClient();
    const agent = guardAgent(
      {
        tools: {
          lookup: {
            id: 'lookup',
            description: 'Lookup data',
            inputSchema: { type: 'object' },
            async execute(input: { id: string }): Promise<string> {
              return input.id;
            },
          },
        },
      },
      { agentId: 'lookup-agent', client },
    );

    await agent.tools.lookup.execute({ id: '1' });

    expect(fetchSpy).toHaveBeenCalledOnce();
    expect(requestUrl(fetchSpy.mock.calls[0]![0])).toBe('http://x/v1/events');
  });

  it('preserves method this binding and avoids double wrapping', async () => {
    const { client, fetchSpy } = permitClient();
    const tool = {
      id: 'counter',
      description: 'Increment a counter',
      inputSchema: { type: 'object' },
      count: 0,
      async execute(input: { amount: number }): Promise<number> {
        this.count += input.amount;
        return this.count;
      },
    };
    const original = { tools: { tool } };
    const guardedOnce = guardAgent(original, { agentId: 'counter-agent', client });
    const guardedTwice = guardAgent(guardedOnce, { agentId: 'counter-agent', client });

    await expect(guardedTwice.tools.tool.execute({ amount: 2 })).resolves.toBe(2);
    expect(tool.count).toBe(2);
    expect(fetchSpy).toHaveBeenCalledOnce();
  });

  it('warns when a provider-hosted tool has no local execute function', () => {
    const warnings: GuardToolDiscoveryWarning[] = [];

    guardAgent(
      {
        tools: [{ type: 'web_search', name: 'web_search' }],
      },
      {
        agentId: 'hosted-tool-agent',
        client: permitClient().client,
        tools: { onDiscoveryWarning: (warning) => warnings.push(warning) },
      },
    );

    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toMatchObject({
      code: 'tool_not_executable',
      framework: 'openai-agents',
      registryKey: '0',
    });
  });

  it('produces the same schema identity regardless of object key order', () => {
    const first = {
      type: 'object',
      properties: {
        city: { type: 'string' },
        units: { enum: ['c', 'f'], type: 'string' },
      },
    };
    const second = {
      properties: {
        units: { type: 'string', enum: ['c', 'f'] },
        city: { type: 'string' },
      },
      type: 'object',
    };

    expect(toolSchemaHash(first)).toBe(toolSchemaHash(second));
  });
});
