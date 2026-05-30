// `guard()` helper tests. Builds a `Client` with a mock `fetchImpl` so
// no network is involved, then verifies each branch of the dispatch.

import { describe, expect, it, vi } from 'vitest';

import {
  Client,
  GuardMode,
  guard,
  Transport,
  Unavailable,
  type Decision,
  type GuardLogEvent,
  type RegenerateFeedback,
} from '../src';
import { mockFetch } from './test-utils';

interface GuardWireRequest {
  agent_id: string;
  channel?: string;
  domain?: string;
  proposed_output: string;
  trace_id?: string;
  context?: {
    docs?: string[];
  };
}

function clientReturning(decision: Partial<Decision>): Client {
  const fetchImpl = mockFetch(async () => {
    return new Response(
      JSON.stringify({
        trace_id: 't-1',
        verdict: 'allow',
        reason: 'ok',
        triggered_policies: [],
        safe_output: null,
        latency_ms: 1,
        tier_results: [],
        ...decision,
      }),
      { status: 200, headers: { 'content-type': 'application/json' } },
    );
  });
  return new Client({ baseUrl: 'http://x', fetchImpl });
}

function clientReturningSequence(decisions: Partial<Decision>[]): {
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
        verdict: 'allow',
        reason: 'ok',
        triggered_policies: [],
        safe_output: null,
        latency_ms: 1,
        tier_results: [],
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
  onEscalate: () => 'CANNED_ESCALATE',
};

describe('guard()', () => {
  it('returns the draft on allow by default', async () => {
    const client = clientReturning({ verdict: 'allow' });
    const out = await guard({ ...DEFAULT_OPTS, client });
    expect(out).toBe('hello there');
  });

  it('returns the safe_output on rewrite by default', async () => {
    const client = clientReturning({
      verdict: 'rewrite',
      safe_output: 'I will connect you with a teammate.',
    });
    const out = await guard({ ...DEFAULT_OPTS, client });
    expect(out).toBe('I will connect you with a teammate.');
  });

  it('falls back to draft on rewrite when no safe_output', async () => {
    const client = clientReturning({ verdict: 'rewrite', safe_output: null });
    const out = await guard({ ...DEFAULT_OPTS, client });
    expect(out).toBe('hello there');
  });

  it('invokes onBlock on block verdict', async () => {
    const client = clientReturning({ verdict: 'block' });
    const onBlock = vi.fn(() => 'BLOCKED');
    const out = await guard({ ...DEFAULT_OPTS, client, onBlock });
    expect(out).toBe('BLOCKED');
    expect(onBlock).toHaveBeenCalledOnce();
    const decision = onBlock.mock.calls[0]![0]!;
    expect(decision.verdict).toBe('block');
  });

  it('invokes onEscalate on escalate verdict', async () => {
    const client = clientReturning({ verdict: 'escalate' });
    const onEscalate = vi.fn(() => 'ESCALATED');
    const out = await guard({ ...DEFAULT_OPTS, client, onEscalate });
    expect(out).toBe('ESCALATED');
    expect(onEscalate).toHaveBeenCalledOnce();
  });

  it('passes through onAllow when supplied', async () => {
    const client = clientReturning({ verdict: 'allow' });
    const onAllow = vi.fn((draft: string) => `[audited] ${draft}`);
    const out = await guard({ ...DEFAULT_OPTS, client, onAllow });
    expect(out).toBe('[audited] hello there');
  });

  it('passes through onRevise when supplied', async () => {
    const client = clientReturning({
      verdict: 'rewrite',
      safe_output: 'sanitised',
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
    const client = clientReturning({ verdict: 'block', trace_id: 'trace-x' });
    const events: GuardLogEvent[] = [];
    await guard({
      ...DEFAULT_OPTS,
      client,
      log: (e) => events.push(e),
    });
    expect(events).toHaveLength(1);
    expect(events[0]!.trace_id).toBe('trace-x');
    expect(events[0]!.verdict).toBe('block');
    expect(events[0]!.branch).toBe('block');
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
          verdict: 'allow',
          reason: 'ok',
          triggered_policies: [],
          safe_output: null,
          latency_ms: 1,
          tier_results: [],
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl: fetchSpy });

    await guard({
      ...DEFAULT_OPTS,
      client,
      channel: 'voice',
      domain: 'voice_agent',
      context: { docs: ['kb-1'] },
      traceId: 'caller-trace-1',
    });

    const call = fetchSpy.mock.calls[0]!;
    const init = call[1];
    if (init === undefined) throw new Error('expected fetch init');
    const body = JSON.parse(init.body as string) as GuardWireRequest;
    expect(body.agent_id).toBe('a');
    expect(body.channel).toBe('voice');
    expect(body.domain).toBe('voice_agent');
    expect(body.proposed_output).toBe('hello there');
    expect(body.trace_id).toBe('caller-trace-1');
    expect(body.context?.docs).toEqual(['kb-1']);
  });

  it('factory form returns an async guard callable', async () => {
    const fetchSpy = mockFetch(async () => {
      return new Response(
        JSON.stringify({
          trace_id: 't',
          verdict: 'allow',
          reason: 'ok',
          triggered_policies: [],
          safe_output: null,
          latency_ms: 1,
          tier_results: [],
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
    const body = JSON.parse(init.body as string) as GuardWireRequest;
    expect(body.agent_id).toBe('factory-agent');
  });

  it('factory form uses default block reply', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ verdict: 'block' }),
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe("I can't help with that request.");
  });

  it('factory form accepts string branch overrides', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ verdict: 'escalate' }),
      onEscalate: 'A human should review this.',
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

  it('factory strict mode blocks rewrite verdicts', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ verdict: 'rewrite', safe_output: 'sanitised' }),
      mode: GuardMode.Strict,
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe("I can't help with that request.");
  });

  it('factory rewrite mode blocks rewrite verdicts without safe output', async () => {
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ verdict: 'rewrite', safe_output: null }),
      mode: GuardMode.Rewrite,
    });

    const out = await guardrail({ input: 'hi', draft: 'unsafe' });
    expect(out).toBe("I can't help with that request.");
  });

  it('factory regenerate mode prefers safe output', async () => {
    const regenerate = vi.fn((_feedback: RegenerateFeedback) => 'regenerated');
    const guardrail = guard({
      agentId: 'factory-agent',
      client: clientReturning({ verdict: 'rewrite', safe_output: 'sanitised' }),
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
        verdict: 'rewrite',
        safe_output: null,
        reason: 'contains confidential data',
        trace_id: 't-1',
      },
      { verdict: 'allow', trace_id: 't-2' },
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
      { verdict: 'rewrite', safe_output: null, trace_id: 't-1' },
      { verdict: 'rewrite', safe_output: null, trace_id: 't-2' },
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
    const { client, fetchSpy } = clientReturningSequence([
      { verdict: 'allow', trace_id: 't-1' },
    ]);
    const guardrail = guard({ agentId: 'stream-agent', client });

    async function* chunks(): AsyncGenerator<string> {
      yield 'Our hours ';
      yield 'are 9 ';
      yield 'to 5.';
    }

    const out = await guardrail.stream({ input: 'when are you open?', draft: chunks() });

    // The full buffered draft is what gets guarded and returned on allow.
    expect(out).toBe('Our hours are 9 to 5.');
    expect(fetchSpy).toHaveBeenCalledOnce();
    const body = JSON.parse(fetchSpy.mock.calls[0]?.[1]?.body as string) as GuardWireRequest;
    expect(body.proposed_output).toBe('Our hours are 9 to 5.');
  });

  it('stream() returns the safe message when the buffered output is blocked', async () => {
    const { client } = clientReturningSequence([{ verdict: 'block', trace_id: 't-1' }]);
    const guardrail = guard({ agentId: 'stream-agent', client });

    async function* chunks(): AsyncGenerator<string> {
      yield 'leak ';
      yield 'the secret';
    }

    const out = await guardrail.stream({ input: 'tell me', draft: chunks() });
    expect(out).toBe("I can't help with that request.");
  });
});
