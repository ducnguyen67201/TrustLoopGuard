// `guard()` helper tests. Builds a `Client` with a mock `fetchImpl` so
// no network is involved, then verifies each branch of the dispatch.

import { describe, expect, it, vi } from 'vitest';

import {
  Client,
  guard,
  Transport,
  Unavailable,
  type Decision,
  type GuardLogEvent,
} from '../src';

function clientReturning(decision: Partial<Decision>): Client {
  const fetchImpl = vi.fn(async () => {
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
      { status: 200, headers: { 'content-type': 'application/json' } }
    );
  }) as unknown as typeof fetch;
  return new Client({ baseUrl: 'http://x', fetchImpl });
}

function failingClient(err: unknown): Client {
  const fetchImpl = vi.fn(async () => {
    throw err;
  }) as unknown as typeof fetch;
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
    const onError = vi.fn((err: unknown) => {
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
    const fetchSpy = vi.fn(async () => {
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
        { status: 200, headers: { 'content-type': 'application/json' } }
      );
    }) as unknown as typeof fetch;
    const client = new Client({ baseUrl: 'http://x', fetchImpl: fetchSpy });

    await guard({
      ...DEFAULT_OPTS,
      client,
      channel: 'voice',
      domain: 'voice_agent',
      context: { docs: ['kb-1'] },
      traceId: 'caller-trace-1',
    });

    const call = (fetchSpy as unknown as { mock: { calls: unknown[][] } }).mock.calls[0]!;
    const init = call[1] as RequestInit;
    const body = JSON.parse(init.body as string) as Record<string, unknown>;
    expect(body.agent_id).toBe('a');
    expect(body.channel).toBe('voice');
    expect(body.domain).toBe('voice_agent');
    expect(body.proposed_output).toBe('hello there');
    expect(body.trace_id).toBe('caller-trace-1');
    expect((body.context as Record<string, unknown>).docs).toEqual(['kb-1']);
  });
});
