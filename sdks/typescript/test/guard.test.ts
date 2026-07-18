// `guard()` helper tests. Builds a `Client` with a mock `fetchImpl` so
// no network is involved, then verifies each branch of the dispatch.

import { describe, expect, it, vi } from 'vitest';

import {
  Client,
  GuardMode,
  guard,
  guardAgent,
  liveKitRun,
  Transport,
  Unavailable,
  type AuthorizationDecision,
  type GuardAgentRunWarning,
  type GuardLogEvent,
  type RegenerateFeedback,
} from '../src';
import { mockFetch } from './test-utils';

interface GuardWireEvent {
  principal: {
    agent_id: string;
    run_id?: string;
  };
  action: {
    parameters: {
      text: string;
    };
  };
  context?: {
    channel?: string;
    domain?: string;
    docs?: string[];
  };
}

interface RunCreateBody {
  agent_id: string;
  kind: string;
  external_id?: string;
  metadata: Record<string, JsonValue>;
}

interface RunUpdateBody {
  status: string;
}

type CapturedBody = GuardWireEvent | RunCreateBody | RunUpdateBody;

interface CapturedRequest {
  url: string;
  method: string;
  body: CapturedBody | null;
}

function runSummary(status = 'running'): Record<string, JsonValue> {
  return {
    id: '018f1111-1111-7111-8111-111111111111',
    workspace_id: 'ws_1',
    environment_id: 'production',
    environment: 'production',
    agent_id: 'support-agent',
    kind: 'chat_session',
    status,
    external_id: null,
    metadata: {},
    started_at: '2026-01-01T00:00:00Z',
    ended_at: status === 'running' ? null : '2026-01-01T00:01:00Z',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:01:00Z',
    trace_count: 0,
    blocked_count: 0,
    rewritten_count: 0,
    escalated_count: 0,
    p95_latency_ms: null,
  };
}

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

interface LiveKitCloseEvent {
  reason?: string;
  error?: Error | null;
}

type LiveKitCloseListener = (event: LiveKitCloseEvent) => void | Promise<void>;

class FakeLiveKitSession {
  private readonly closeListeners = new Set<LiveKitCloseListener>();

  on(event: 'close', listener: LiveKitCloseListener): this {
    if (event === 'close') this.closeListeners.add(listener);
    return this;
  }

  off(event: 'close', listener: LiveKitCloseListener): this {
    if (event === 'close') this.closeListeners.delete(listener);
    return this;
  }

  async close(event: LiveKitCloseEvent): Promise<void> {
    await Promise.all([...this.closeListeners].map((listener) => listener(event)));
  }

  listenerCount(): number {
    return this.closeListeners.size;
  }
}

interface Deferred<Value> {
  promise: Promise<Value>;
  resolve(value: Value): void;
}

function deferred<Value>(): Deferred<Value> {
  let resolvePromise: (value: Value) => void = () => {
    throw new Error('deferred resolver was not initialized');
  };
  const promise = new Promise<Value>((resolve) => {
    resolvePromise = resolve;
  });
  return { promise, resolve: resolvePromise };
}

function automaticRunClient(failAt?: 'start' | 'finish'): {
  client: Client;
  requests: CapturedRequest[];
} {
  const requests: CapturedRequest[] = [];
  const fetchImpl = mockFetch(async (input, init) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
    const method = init?.method ?? 'GET';
    const body = init?.body ? (JSON.parse(String(init.body)) as CapturedBody) : null;
    requests.push({ url, method, body });

    if (url === 'http://x/v1/runs' && method === 'POST') {
      if (failAt === 'start') {
        return Response.json(
          { code: 'internal', message: 'run start failed', retriable: false },
          { status: 500 },
        );
      }
      return Response.json(runSummary(), { status: 201 });
    }
    if (url === 'http://x/v1/runs/018f1111-1111-7111-8111-111111111111' && method === 'PATCH') {
      if (failAt === 'finish') {
        return Response.json(
          { code: 'internal', message: 'run finish failed', retriable: false },
          { status: 500 },
        );
      }
      const nextStatus = body !== null && 'status' in body ? body.status : 'completed';
      return Response.json(runSummary(nextStatus));
    }
    return Response.json({
      trace_id: 't-1',
      effect: 'permit',
      reason: 'ok',
      findings: [],
      transformed_value: null,
      latency_ms: 1,
    });
  });
  return {
    client: new Client({ baseUrl: 'http://x', fetchImpl }),
    requests,
  };
}

function clientReturning(decision: Partial<AuthorizationDecision>): Client {
  const fetchImpl = mockFetch(async () => {
    return new Response(
      JSON.stringify({
        trace_id: 't-1',
        effect: 'permit',
        reason: 'ok',
        findings: [],
        transformed_value: null,
        latency_ms: 1,
        ...decision,
      }),
      { status: 200, headers: { 'content-type': 'application/json' } },
    );
  });
  return new Client({ baseUrl: 'http://x', fetchImpl });
}

function clientReturningSequence(decisions: Partial<AuthorizationDecision>[]): {
  client: Client;
  fetchSpy: ReturnType<typeof mockFetch>;
} {
  const pending = [...decisions];
  const fetchSpy = mockFetch(async () => {
    const decision = pending.shift();
    if (decision === undefined) throw new Error('no mock decision left');
    return new Response(
      JSON.stringify({
        trace_id: 't-1',
        effect: 'permit',
        reason: 'ok',
        findings: [],
        transformed_value: null,
        latency_ms: 1,
        ...decision,
      }),
      { status: 200, headers: { 'content-type': 'application/json' } },
    );
  });
  return {
    client: new Client({ baseUrl: 'http://x', fetchImpl: fetchSpy }),
    fetchSpy,
  };
}

function failingClient(err: Error): Client {
  const fetchImpl = mockFetch(async () => {
    throw err;
  });
  return new Client({
    baseUrl: 'http://x',
    fetchImpl,
    retry: { maxAttempts: 1, baseS: 0, capS: 0, totalBudgetS: 0 },
  });
}

const DEFAULT_OPTS = {
  input: 'hi',
  draft: 'hello there',
  agentId: 'a',
  onBlock: () => 'CANNED_BLOCK',
  onRequireApproval: () => 'CANNED_APPROVAL',
  onDefer: () => 'CANNED_DEFER',
};

describe('guard()', () => {
  it('returns the draft on permit by default', async () => {
    const client = clientReturning({ effect: 'permit' });
    const out = await guard({ ...DEFAULT_OPTS, client });
    expect(out).toBe('hello there');
  });

  it('returns the transformed_value on transform by default', async () => {
    const client = clientReturning({
      effect: 'transform',
      transformed_value: 'I will connect you with a teammate.',
    });
    const out = await guard({ ...DEFAULT_OPTS, client });
    expect(out).toBe('I will connect you with a teammate.');
  });

  it('falls back to draft on transform when no transformed_value', async () => {
    const client = clientReturning({ effect: 'transform', transformed_value: null });
    const out = await guard({ ...DEFAULT_OPTS, client });
    expect(out).toBe('hello there');
  });

  it('invokes onBlock on deny effect', async () => {
    const client = clientReturning({ effect: 'deny' });
    const onBlock = vi.fn(() => 'BLOCKED');
    const out = await guard({ ...DEFAULT_OPTS, client, onBlock });
    expect(out).toBe('BLOCKED');
    expect(onBlock).toHaveBeenCalledOnce();
    const decision = onBlock.mock.calls[0]![0]!;
    expect(decision.effect).toBe('deny');
  });

  it('invokes onRequireApproval on require_approval effect', async () => {
    const client = clientReturning({ effect: 'require_approval' });
    const onRequireApproval = vi.fn(() => 'ESCALATED');
    const out = await guard({ ...DEFAULT_OPTS, client, onRequireApproval });
    expect(out).toBe('ESCALATED');
    expect(onRequireApproval).toHaveBeenCalledOnce();
  });

  it('invokes onDefer without treating missing evidence as approval', async () => {
    const client = clientReturning({ effect: 'defer' });
    const onDefer = vi.fn(() => 'RETRY_LATER');
    const onRequireApproval = vi.fn(() => 'REVIEW');
    const out = await guard({ ...DEFAULT_OPTS, client, onDefer, onRequireApproval });
    expect(out).toBe('RETRY_LATER');
    expect(onDefer).toHaveBeenCalledOnce();
    expect(onRequireApproval).not.toHaveBeenCalled();
  });

  it('passes through onAllow when supplied', async () => {
    const client = clientReturning({ effect: 'permit' });
    const onAllow = vi.fn((draft: string) => `[audited] ${draft}`);
    const out = await guard({ ...DEFAULT_OPTS, client, onAllow });
    expect(out).toBe('[audited] hello there');
  });

  it('passes through onRevise when supplied', async () => {
    const client = clientReturning({
      effect: 'transform',
      transformed_value: 'sanitised',
    });
    const onRevise = vi.fn((revised: string | null) => `${revised}!`);
    const out = await guard({ ...DEFAULT_OPTS, client, onRevise });
    expect(out).toBe('sanitised!');
  });

  it('fails open by default on transport error', async () => {
    const client = failingClient(new Error('econnreset'));
    const out = await guard({ ...DEFAULT_OPTS, client });
    expect(out).toBe('hello there');
  });

  it('routes errors through onError when supplied', async () => {
    const client = failingClient(new Error('boom'));
    const onError = vi.fn((err) => {
      expect(err).toBeInstanceOf(Transport);
      return 'FAIL_CLOSED';
    });
    const out = await guard({ ...DEFAULT_OPTS, client, onError });
    expect(out).toBe('FAIL_CLOSED');
  });

  it('emits a log event with the chosen branch', async () => {
    const client = clientReturning({ effect: 'deny', trace_id: 'trace-x' });
    const events: GuardLogEvent[] = [];
    await guard({
      ...DEFAULT_OPTS,
      client,
      log: (e) => events.push(e),
    });
    expect(events).toHaveLength(1);
    expect(events[0]!.trace_id).toBe('trace-x');
    expect(events[0]!.effect).toBe('deny');
    expect(events[0]!.branch).toBe('deny');
  });

  it('logs branch="error" on transport failure', async () => {
    const client = failingClient(new Unavailable('upstream'));
    const events: GuardLogEvent[] = [];
    await guard({
      ...DEFAULT_OPTS,
      client,
      log: (e) => events.push(e),
    });
    expect(events).toHaveLength(1);
    expect(events[0]!.branch).toBe('error');
  });

  it('builds the wire request shape correctly', async () => {
    const fetchSpy = mockFetch(async () => {
      return new Response(
        JSON.stringify({
          trace_id: 't',
          effect: 'permit',
          reason: 'ok',
          findings: [],
          transformed_value: null,
          latency_ms: 1,
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl: fetchSpy });

    await guard({
      ...DEFAULT_OPTS,
      input: 'private-user-input',
      client,
      channel: 'voice',
      domain: 'voice_agent',
      context: { docs: ['kb-1'] },
      traceId: 'caller-trace-1',
    });

    const call = fetchSpy.mock.calls[0]!;
    const init = call[1];
    if (init === undefined) throw new Error('expected fetch init');
    const body = JSON.parse(init.body as string) as GuardWireEvent;
    expect(body.principal.agent_id).toBe('a');
    expect(body.context?.channel).toBe('voice');
    expect(body.context?.domain).toBe('voice_agent');
    expect(body.action.parameters.text).toBe('hello there');
    expect(body.context?.docs).toEqual(['kb-1']);
    expect(JSON.stringify(body)).not.toContain('private-user-input');
  });

  it('factory form returns an async guard callable', async () => {
    const fetchSpy = mockFetch(async () => {
      return new Response(
        JSON.stringify({
          trace_id: 't',
          effect: 'permit',
          reason: 'ok',
          findings: [],
          transformed_value: null,
          latency_ms: 1,
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });

    const guardrail = guard({
      agentId: 'factory-agent',
      baseUrl: 'http://x',
      fetchImpl: fetchSpy,
    });

    const out = await guardrail({ input: 'hi', draft: 'hello' });
    expect(out).toBe('hello');

    const call = fetchSpy.mock.calls[0]!;
    const init = call[1];
    if (init === undefined) throw new Error('expected fetch init');
    const body = JSON.parse(init.body as string) as GuardWireEvent;
    expect(body.principal.agent_id).toBe('factory-agent');
  });

  it('factory form reads TLG_URL and TLG_API_KEY from the environment', async () => {
    const previousUrl = process.env['TLG_URL'];
    const previousKey = process.env['TLG_API_KEY'];
    process.env['TLG_URL'] = 'https://api.example.test';
    process.env['TLG_API_KEY'] = 'test-runtime-key';
    const fetchSpy = mockFetch(async () => {
      return new Response(
        JSON.stringify({
          trace_id: 't',
          effect: 'permit',
          reason: 'ok',
          findings: [],
          transformed_value: null,
          latency_ms: 1,
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });

    try {
      const protect = guard({ agentId: 'env-agent', fetchImpl: fetchSpy });
      await protect({ input: 'hello', draft: 'reply' });
    } finally {
      if (previousUrl === undefined) delete process.env['TLG_URL'];
      else process.env['TLG_URL'] = previousUrl;
      if (previousKey === undefined) delete process.env['TLG_API_KEY'];
      else process.env['TLG_API_KEY'] = previousKey;
    }

    expect(fetchSpy.mock.calls[0]?.[0]).toBe('https://api.example.test/v1/events');
    const headers = fetchSpy.mock.calls[0]?.[1]?.headers as Record<string, string>;
    expect(headers['authorization']).toBe('Bearer test-runtime-key');
  });

  it('wrap() turns an async agent function into a guarded function', async () => {
    const fetchSpy = mockFetch(async () => {
      return new Response(
        JSON.stringify({
          trace_id: 't',
          effect: 'permit',
          reason: 'ok',
          findings: [],
          transformed_value: null,
          latency_ms: 1,
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });
    const protect = guard({
      agentId: 'wrapped-agent',
      baseUrl: 'http://x',
      fetchImpl: fetchSpy,
    });
    const answer = protect.wrap(async (message: string) => `reply to ${message}`);

    const out = await answer('hello');

    expect(out).toBe('reply to hello');
    const body = JSON.parse(fetchSpy.mock.calls[0]?.[1]?.body as string) as GuardWireEvent;
    expect(body.principal.agent_id).toBe('wrapped-agent');
    expect(body.action.parameters.text).toBe('reply to hello');
  });

  it('wrap() accepts an input selector for functions with structured arguments', async () => {
    const { client, fetchSpy } = clientReturningSequence([{ effect: 'permit', trace_id: 't-1' }]);
    const protect = guard({ agentId: 'wrapped-agent', client });
    const answer = protect.wrap(
      async (request: { message: string; locale: string }) =>
        `${request.locale}: ${request.message}`,
      { input: (request) => request.message },
    );

    const out = await answer({ message: 'hello', locale: 'en' });

    expect(out).toBe('en: hello');
    const body = JSON.parse(fetchSpy.mock.calls[0]?.[1]?.body as string) as GuardWireEvent;
    expect(body.action.parameters.text).toBe('en: hello');
  });

  it('wrap() rejects a non-string inferred input before calling the agent', async () => {
    const { client, fetchSpy } = clientReturningSequence([{ effect: 'permit', trace_id: 't-1' }]);
    const protect = guard({ agentId: 'wrapped-agent', client });
    const agent = vi.fn(async (request: { message: string }) => request.message);
    const answer = protect.wrap(agent);

    await expect(answer({ message: 'hello' })).rejects.toThrow(
      'guard.wrap() input must be a string',
    );
    expect(agent).not.toHaveBeenCalled();
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('wrap() rejects a non-string agent result without submitting an event', async () => {
    const { client, fetchSpy } = clientReturningSequence([{ effect: 'permit', trace_id: 't-1' }]);
    const protect = guard({ agentId: 'wrapped-agent', client });
    const answer = protect.wrap(async (message: string) => ({ message }));

    await expect(answer('hello')).rejects.toThrow(
      'guard.wrap() wrapped function must return a string',
    );
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('wrap() fails closed by default on transport errors', async () => {
    const protect = guard({
      agentId: 'wrapped-agent',
      client: failingClient(new Unavailable('upstream')),
    });
    const answer = protect.wrap(async (message: string) => `reply to ${message}`);

    const out = await answer('hello');

    expect(out).toBe("I can't help with that request.");
  });

  it('factory form uses default deny reply', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ effect: 'deny' }),
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe("I can't help with that request.");
  });

  it('factory form accepts string branch overrides', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ effect: 'require_approval' }),
      onRequireApproval: 'A human should review this.',
    });

    const out = await guardrail({ input: 'hi', draft: 'needs review' });
    expect(out).toBe('A human should review this.');
  });

  it('factory form can fail closed on transport errors', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: failingClient(new Unavailable('upstream')),
      failClosed: true,
    });

    const out = await guardrail({ input: 'hi', draft: 'original' });
    expect(out).toBe("I can't help with that request.");
  });

  it('factory strict mode blocks transform effects', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ effect: 'transform', transformed_value: 'sanitised' }),
      mode: GuardMode.Strict,
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe("I can't help with that request.");
  });

  it('factory transform mode blocks transform effects without safe output', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ effect: 'transform', transformed_value: null }),
      mode: GuardMode.Rewrite,
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe("I can't help with that request.");
  });

  it('factory regenerate mode prefers safe output', async () => {
    const regenerate = vi.fn((_feedback: RegenerateFeedback) => 'regenerated');
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ effect: 'transform', transformed_value: 'sanitised' }),
      mode: GuardMode.RewriteOrRegenerate,
      regenerate,
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe('sanitised');
    expect(regenerate).not.toHaveBeenCalled();
  });

  it('factory regenerate mode asks the model to retry and checks again', async () => {
    const { client, fetchSpy } = clientReturningSequence([
      {
        effect: 'transform',
        transformed_value: null,
        reason: 'contains confidential data',
        trace_id: 't-1',
      },
      { effect: 'permit', trace_id: 't-2' },
    ]);
    const seen: RegenerateFeedback[] = [];
    const regenerate = vi.fn((feedback: RegenerateFeedback) => {
      seen.push(feedback);
      return 'safer regenerated reply';
    });
    const guardrail = guard({
      agentId: 'factory-agent',
      client,
      mode: GuardMode.RewriteOrRegenerate,
      regenerate,
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe('safer regenerated reply');
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(seen[0]!.reason).toBe('contains confidential data');
    expect(seen[0]!.attempt).toBe(1);
    expect(seen[0]!.maxAttempts).toBe(1);
  });

  it('factory regenerate mode caps retries', async () => {
    const { client, fetchSpy } = clientReturningSequence([
      { effect: 'transform', transformed_value: null, trace_id: 't-1' },
      { effect: 'transform', transformed_value: null, trace_id: 't-2' },
    ]);
    const regenerate = vi.fn(() => 'still unsafe');
    const guardrail = guard({
      agentId: 'factory-agent',
      client,
      mode: GuardMode.RewriteOrRegenerate,
      regenerate,
      maxRegenerations: 1,
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe("I can't help with that request.");
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(regenerate).toHaveBeenCalledOnce();
  });

  it('stream() buffers a chunk stream then guards the full output', async () => {
    const { client, fetchSpy } = clientReturningSequence([{ effect: 'permit', trace_id: 't-1' }]);
    const guardrail = guard({ agentId: 'stream-agent', client });

    async function* chunks(): AsyncGenerator<string> {
      yield 'Our hours ';
      yield 'are 9 ';
      yield 'to 5.';
    }

    const out = await guardrail.stream({ input: 'when are you open?', draft: chunks() });

    // The full buffered draft is what gets guarded and returned on permit.
    expect(out).toBe('Our hours are 9 to 5.');
    expect(fetchSpy).toHaveBeenCalledOnce();
    const body = JSON.parse(fetchSpy.mock.calls[0]?.[1]?.body as string) as GuardWireEvent;
    expect(body.action.parameters.text).toBe('Our hours are 9 to 5.');
  });

  it('stream() returns the safe message when the buffered output is blocked', async () => {
    const { client } = clientReturningSequence([{ effect: 'deny', trace_id: 't-1' }]);
    const guardrail = guard({ agentId: 'stream-agent', client });

    async function* chunks(): AsyncGenerator<string> {
      yield 'leak ';
      yield 'the secret';
    }

    const out = await guardrail.stream({ input: 'tell me', draft: chunks() });
    expect(out).toBe("I can't help with that request.");
  });
});

describe('guardAgent()', () => {
  it('creates, links, and completes one run for every reply by default', async () => {
    const { client, requests } = automaticRunClient();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `reply to ${message}`;
        },
      },
      { agentId: 'support-agent', client },
    );

    const first = await agent.reply('hello');
    const second = await agent.reply('again');

    expect([first, second]).toEqual(['reply to hello', 'reply to again']);
    expect(requests.map(({ url, method }) => `${method} ${url}`)).toEqual([
      'POST http://x/v1/runs',
      'POST http://x/v1/events',
      'PATCH http://x/v1/runs/018f1111-1111-7111-8111-111111111111',
      'POST http://x/v1/runs',
      'POST http://x/v1/events',
      'PATCH http://x/v1/runs/018f1111-1111-7111-8111-111111111111',
    ]);
    expect(requests[0]?.body).toEqual({
      agent_id: 'support-agent',
      kind: 'chat_session',
      metadata: { integration: 'guardAgent' },
    });
    expect(requests[3]?.body).toEqual(requests[0]?.body);
    expect((requests[1]?.body as GuardWireEvent).principal.run_id).toBe(
      '018f1111-1111-7111-8111-111111111111',
    );
    expect((requests[4]?.body as GuardWireEvent).principal.run_id).toBe(
      '018f1111-1111-7111-8111-111111111111',
    );
    expect(requests[2]?.body).toEqual({ status: 'completed' });
    expect(requests[5]?.body).toEqual({ status: 'completed' });
  });

  it('keeps one run open across a LiveKit session and reuses it for every reply', async () => {
    const { client, requests } = automaticRunClient();
    const session = new FakeLiveKitSession();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return 'reply to ' + message;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, {
          externalId: 'RM_session_1',
          metadata: { tenant: 'north' },
        }),
      },
    );

    await expect(agent.reply('first secret message')).resolves.toBe(
      'reply to first secret message',
    );
    await expect(agent.reply('second')).resolves.toBe('reply to second');

    expect(requests.map(({ url, method }) => method + ' ' + url)).toEqual([
      'POST http://x/v1/runs',
      'POST http://x/v1/events',
      'POST http://x/v1/events',
    ]);
    expect(requests[0]?.body).toEqual({
      agent_id: 'support-agent',
      kind: 'live_call',
      external_id: 'RM_session_1',
      metadata: { integration: 'guardAgent', tenant: 'north' },
    });
    const events = requests
      .filter(({ url }) => url === 'http://x/v1/events')
      .map(({ body }) => body as GuardWireEvent);
    expect(events.map(({ principal }) => principal.run_id)).toEqual([
      '018f1111-1111-7111-8111-111111111111',
      '018f1111-1111-7111-8111-111111111111',
    ]);
    expect(JSON.stringify(requests[0]?.body)).not.toContain('first secret message');
    expect(session.listenerCount()).toBe(1);

    await session.close({ reason: 'task_completed' });

    expect(requests.at(-1)).toMatchObject({
      method: 'PATCH',
      body: { status: 'completed' },
    });
    expect(session.listenerCount()).toBe(0);
  });

  it('deduplicates concurrent session run creation and finishes after the start resolves', async () => {
    const requests: CapturedRequest[] = [];
    const startSeen = deferred<void>();
    const startResponse = deferred<Response>();
    const fetchImpl = mockFetch(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      const method = init?.method ?? 'GET';
      const body = init?.body ? (JSON.parse(String(init.body)) as CapturedBody) : null;
      requests.push({ url, method, body });

      if (url === 'http://x/v1/runs' && method === 'POST') {
        startSeen.resolve(undefined);
        return await startResponse.promise;
      }
      if (method === 'PATCH') return Response.json(runSummary('completed'));
      return Response.json({
        trace_id: 't-1',
        effect: 'permit',
        reason: 'ok',
        findings: [],
        transformed_value: null,
        latency_ms: 1,
      });
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl });
    const session = new FakeLiveKitSession();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return 'reply to ' + message;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, { externalId: async () => 'RM_concurrent' }),
      },
    );

    const first = agent.reply('first');
    const second = agent.reply('second');
    await startSeen.promise;
    const close = session.close({ reason: 'participant_disconnected' });

    expect(
      requests.filter(({ url, method }) => url === 'http://x/v1/runs' && method === 'POST'),
    ).toHaveLength(1);

    startResponse.resolve(Response.json(runSummary(), { status: 201 }));
    await expect(Promise.all([first, second])).resolves.toEqual([
      'reply to first',
      'reply to second',
    ]);
    await close;

    expect(requests.filter(({ method }) => method === 'PATCH').map(({ body }) => body)).toEqual([
      { status: 'completed' },
    ]);
    expect(
      requests
        .filter(({ url }) => url === 'http://x/v1/events')
        .map(({ body }) => (body as GuardWireEvent).principal.run_id),
    ).toEqual(['018f1111-1111-7111-8111-111111111111', '018f1111-1111-7111-8111-111111111111']);
  });

  it('does not create a run when the session ends before guarded activity', async () => {
    const { client, requests } = automaticRunClient();
    const session = new FakeLiveKitSession();

    guardAgent(
      {
        async reply(message: string): Promise<string> {
          return message;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, { externalId: 'RM_idle' }),
      },
    );

    await session.close({ reason: 'user_initiated' });
    await session.close({ reason: 'user_initiated' });

    expect(requests).toEqual([]);
    expect(session.listenerCount()).toBe(0);
  });

  it.each([
    ['error', new Error('model failed'), 'failed'],
    ['job_shutdown', null, 'canceled'],
    ['participant_disconnected', null, 'completed'],
    ['user_initiated', null, 'completed'],
    ['task_completed', null, 'completed'],
    ['future_reason', new Error('future failure'), 'failed'],
    ['future_reason', null, 'completed'],
  ])('maps LiveKit close reason %s with error %s to %s', async (reason, error, expectedStatus) => {
    const { client, requests } = automaticRunClient();
    const session = new FakeLiveKitSession();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return message;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, { externalId: 'RM_status' }),
      },
    );

    await agent.reply('hello');
    await session.close({ reason, error });
    await session.close({ reason, error });

    expect(requests.filter(({ method }) => method === 'PATCH')).toHaveLength(1);
    expect(requests.at(-1)?.body).toEqual({ status: expectedStatus });
  });

  it('rejects an empty session external id without falling back to agent id', async () => {
    const { client, requests } = automaticRunClient();
    const session = new FakeLiveKitSession();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return message;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, { externalId: async () => '   ' }),
      },
    );

    await expect(agent.reply('hello')).rejects.toThrow(
      'guardAgent session run externalId must be a non-empty string',
    );
    expect(requests).toEqual([]);
  });

  it('retries a failed session start only on a later independent boundary', async () => {
    const requests: CapturedRequest[] = [];
    const warnings: GuardAgentRunWarning[] = [];
    let startAttempts = 0;
    const fetchImpl = mockFetch(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      const method = init?.method ?? 'GET';
      const body = init?.body ? (JSON.parse(String(init.body)) as CapturedBody) : null;
      requests.push({ url, method, body });

      if (url === 'http://x/v1/runs' && method === 'POST') {
        startAttempts += 1;
        if (startAttempts === 1) {
          return Response.json(
            { code: 'internal', message: 'run start failed', retriable: false },
            { status: 500 },
          );
        }
        return Response.json(runSummary(), { status: 201 });
      }
      if (method === 'PATCH') return Response.json(runSummary('completed'));
      return Response.json({
        trace_id: 't-1',
        effect: 'permit',
        reason: 'ok',
        findings: [],
        transformed_value: null,
        latency_ms: 1,
      });
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl });
    const session = new FakeLiveKitSession();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return message;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, {
          externalId: 'RM_retry',
          onLifecycleWarning: (warning) => warnings.push(warning),
        }),
      },
    );

    await expect(agent.reply('first')).resolves.toBe('first');
    await expect(agent.reply('second')).resolves.toBe('second');
    await session.close({ reason: 'task_completed' });

    expect(
      requests.filter(({ url, method }) => url === 'http://x/v1/runs' && method === 'POST'),
    ).toHaveLength(2);
    const events = requests
      .filter(({ url }) => url === 'http://x/v1/events')
      .map(({ body }) => body as GuardWireEvent);
    expect(events[0]?.principal.run_id).toBeUndefined();
    expect(events[1]?.principal.run_id).toBe('018f1111-1111-7111-8111-111111111111');
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toMatchObject({ code: 'run_start_failed', phase: 'start' });
  });

  it('preserves an agent error without failing the long-lived session run', async () => {
    const { client, requests } = automaticRunClient();
    const session = new FakeLiveKitSession();
    const agent = guardAgent(
      {
        async reply(_message: string): Promise<string> {
          throw new Error('agent failed');
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, { externalId: 'RM_error' }),
      },
    );

    await expect(agent.reply('hello')).rejects.toThrow('agent failed');
    expect(requests.map(({ method }) => method)).toEqual(['POST']);

    await session.close({ reason: 'task_completed' });
    expect(requests.at(-1)?.body).toEqual({ status: 'completed' });
  });

  it('reuses an explicit run instead of creating a nested run', async () => {
    const { client, requests } = automaticRunClient();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `reply to ${message}`;
        },
      },
      { agentId: 'support-agent', client },
    );

    await client.withRun({ agentId: 'support-agent', kind: 'workflow' }, () =>
      agent.reply('hello'),
    );

    expect(
      requests.filter(({ url, method }) => url === 'http://x/v1/runs' && method === 'POST'),
    ).toHaveLength(1);
    const event = requests.find(({ url }) => url === 'http://x/v1/events');
    expect((event?.body as GuardWireEvent).principal.run_id).toBe(
      '018f1111-1111-7111-8111-111111111111',
    );
  });

  it('lets an explicit run win inside a configured session boundary', async () => {
    const { client, requests } = automaticRunClient();
    const session = new FakeLiveKitSession();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `reply to ${message}`;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, { externalId: 'RM_explicit' }),
      },
    );

    await client.withRun({ agentId: 'support-agent', kind: 'workflow' }, () =>
      agent.reply('hello'),
    );
    await session.close({ reason: 'task_completed' });

    expect(
      requests.filter(({ url, method }) => url === 'http://x/v1/runs' && method === 'POST'),
    ).toHaveLength(1);
    expect(requests.filter(({ method }) => method === 'PATCH')).toHaveLength(1);
    const event = requests.find(({ url }) => url === 'http://x/v1/events');
    expect((event?.body as GuardWireEvent).principal.run_id).toBe(
      '018f1111-1111-7111-8111-111111111111',
    );
  });

  it('allows automatic runs to be disabled without changing reply call sites', async () => {
    const { client, requests } = automaticRunClient();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `reply to ${message}`;
        },
      },
      { agentId: 'support-agent', client, run: false },
    );

    const reply = await agent.reply('hello');

    expect(reply).toBe('reply to hello');
    expect(requests.map(({ url }) => url)).toEqual(['http://x/v1/events']);
  });

  it('marks the automatic run failed and preserves an agent error', async () => {
    const { client, requests } = automaticRunClient();
    const agent = guardAgent(
      {
        async reply(_message: string): Promise<string> {
          throw new Error('agent failed');
        },
      },
      { agentId: 'support-agent', client },
    );

    await expect(agent.reply('hello')).rejects.toThrow('agent failed');

    expect(requests.map(({ url, method }) => `${method} ${url}`)).toEqual([
      'POST http://x/v1/runs',
      'PATCH http://x/v1/runs/018f1111-1111-7111-8111-111111111111',
    ]);
    expect(requests[1]?.body).toEqual({ status: 'failed' });
  });

  it('keeps guard enforcement available when automatic run creation fails', async () => {
    const { client, requests } = automaticRunClient('start');
    const warnings: GuardAgentRunWarning[] = [];
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `reply to ${message}`;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: { onLifecycleWarning: (warning) => warnings.push(warning) },
      },
    );

    const reply = await agent.reply('hello');

    expect(reply).toBe('reply to hello');
    expect(requests.map(({ url }) => url)).toEqual(['http://x/v1/runs', 'http://x/v1/events']);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toMatchObject({ code: 'run_start_failed', phase: 'start' });
  });

  it('does not hide a guarded reply when automatic run completion fails', async () => {
    const { client, requests } = automaticRunClient('finish');
    const warnings: GuardAgentRunWarning[] = [];
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `reply to ${message}`;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: { onLifecycleWarning: (warning) => warnings.push(warning) },
      },
    );

    const reply = await agent.reply('hello');

    expect(reply).toBe('reply to hello');
    expect(requests.map(({ url, method }) => `${method} ${url}`)).toEqual([
      'POST http://x/v1/runs',
      'POST http://x/v1/events',
      'PATCH http://x/v1/runs/018f1111-1111-7111-8111-111111111111',
    ]);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toMatchObject({ code: 'run_finish_failed', phase: 'finish' });
  });

  it('does not hide a completed session when its terminal update fails', async () => {
    const { client, requests } = automaticRunClient('finish');
    const warnings: GuardAgentRunWarning[] = [];
    const session = new FakeLiveKitSession();
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `reply to ${message}`;
        },
      },
      {
        agentId: 'support-agent',
        client,
        run: liveKitRun(session, {
          externalId: 'RM_finish_failure',
          onLifecycleWarning: (warning) => warnings.push(warning),
        }),
      },
    );

    await expect(agent.reply('hello')).resolves.toBe('reply to hello');
    await expect(session.close({ reason: 'task_completed' })).resolves.toBeUndefined();

    expect(requests.filter(({ method }) => method === 'PATCH')).toHaveLength(1);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toMatchObject({ code: 'run_finish_failed', phase: 'finish' });
  });

  it('decorates an agent once while preserving its reply call site and other members', async () => {
    const { client, fetchSpy } = clientReturningSequence([{ effect: 'permit', trace_id: 't-1' }]);

    class SupportAgent {
      #replyCount = 0;
      readonly name = 'support';

      get replyCount(): number {
        return this.#replyCount;
      }

      async reply(message: string): Promise<string> {
        this.#replyCount += 1;
        return `${this.name}: ${message}`;
      }

      status(): string {
        return `${this.name}:${this.replyCount}`;
      }
    }

    const original = new SupportAgent();
    const agent = guardAgent(original, { agentId: 'support-agent', client, run: false });

    const reply = await agent.reply('hello');

    expect(reply).toBe('support: hello');
    expect(agent.name).toBe('support');
    expect(agent.status()).toBe('support:1');
    expect(original.replyCount).toBe(1);
    expect(fetchSpy).toHaveBeenCalledOnce();
    const body = JSON.parse(fetchSpy.mock.calls[0]?.[1]?.body as string) as GuardWireEvent;
    expect(body.principal.agent_id).toBe('support-agent');
    expect(body.action.parameters.text).toBe('support: hello');
  });

  it('returns TrustLoopGuard transformed output from the same reply method', async () => {
    const client = clientReturning({
      effect: 'transform',
      transformed_value: 'A safer response.',
    });
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `unsafe: ${message}`;
        },
      },
      { agentId: 'support-agent', client, run: false },
    );

    const reply = await agent.reply('hello');

    expect(reply).toBe('A safer response.');
  });

  it('forwards additional reply arguments without changing the agent interface', async () => {
    const client = clientReturning({ effect: 'permit' });
    const agent = guardAgent(
      {
        async reply(message: string, locale: string): Promise<string> {
          return `${locale}: ${message}`;
        },
      },
      { agentId: 'support-agent', client, run: false },
    );

    const reply = await agent.reply('hello', 'en');

    expect(reply).toBe('en: hello');
  });

  it('fails closed by default when the guard service is unavailable', async () => {
    const agent = guardAgent(
      {
        async reply(message: string): Promise<string> {
          return `reply to ${message}`;
        },
      },
      {
        agentId: 'support-agent',
        client: failingClient(new Unavailable('upstream')),
        run: false,
      },
    );

    const reply = await agent.reply('hello');

    expect(reply).toBe("I can't help with that request.");
  });
});
