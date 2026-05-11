// `guard()` — one-line integration helper.
//
// Customers integrating TrustLoopGuard go from ~30 lines of branching
// to one call:
//
//   const reply = await guard({
//     client,
//     input,
//     draft,
//     context,
//     agentId: 'acme-support-v3',
//     onBlock:    () => cannedSafeReply,
//     onEscalate: () => { humanQueue.push(...); return holdMessage; },
//   });
//   await sendToCustomer(reply);
//
// The handler stays the source of truth on what to do per-verdict.
// `guard` just makes the dispatch ergonomic and applies a fail-open
// default for transport errors so an outage on our side doesn't take
// down the agent.

import type { Client } from './client';
import type { Channel } from './generated/Channel';
import type { CheckRequest } from './generated/CheckRequest';
import type { Decision } from './generated/Decision';
import { SdkError } from './errors';

export interface GuardCallbacks {
  /**
   * Called when the verdict is `allow`. Default: return the original
   * draft unchanged. Override only if you want to log the allow path
   * or strip a draft suffix etc.
   */
  onAllow?: (draft: string, decision: Decision) => string | Promise<string>;

  /**
   * Called when the verdict is `rewrite`. Default: return
   * `decision.safe_output ?? draft`. Override to post-process or
   * substitute your own canned rewrite.
   */
  onRevise?: (revised: string | null, draft: string, decision: Decision) => string | Promise<string>;

  /**
   * Called when the verdict is `block`. **Required** — there is no
   * sensible automatic answer. Return the canned safe message your
   * brand wants the customer to see (or throw to abort the send).
   */
  onBlock: (decision: Decision) => string | Promise<string>;

  /**
   * Called when the verdict is `escalate`. **Required** — typically
   * pushes onto a human-review queue and returns a holding message.
   */
  onEscalate: (decision: Decision) => string | Promise<string>;

  /**
   * Called when the SDK transport itself fails (network down, server
   * 5xx, decode error, retries exhausted). Default: **fail-open** —
   * return the original draft. Pass an explicit handler if you'd
   * rather fail-closed (e.g. `() => cannedSafeReply`).
   */
  onError?: (err: SdkError, draft: string) => string | Promise<string>;
}

export interface GuardOptions extends GuardCallbacks {
  client: Client;

  /** What the user said. */
  input: string;

  /** What the agent wants to send. The string returned by `guard` is
   *  what the caller should actually deliver. */
  draft: string;

  /** Conversation channel — drives latency budget on the server. */
  channel?: Channel;

  /** Required: the registered agent profile id. */
  agentId: string;

  /**
   * Optional structured context — typically `{ docs: [...] }` for
   * grounding the LLM judges. Anything JSON-serialisable.
   */
  context?: Record<string, unknown>;

  /**
   * Optional override for `domain` (defaults to the server's
   * `customer_support` dispatcher).
   */
  domain?: string;

  /** Optional caller-supplied trace id — overrides the server-assigned one. */
  traceId?: string;

  /**
   * Logger hook. If provided, gets one structured event per `guard`
   * invocation: { trace_id, verdict, branch, latency_ms }. Useful for
   * surfacing which branch fired without forcing a specific logger
   * dependency.
   */
  log?: (event: GuardLogEvent) => void;

  /** Optional cancellation. Forwarded to the underlying check call. */
  signal?: AbortSignal;
}

export interface GuardLogEvent {
  trace_id: string;
  verdict: Decision['verdict'];
  /** Which callback we ended up calling. */
  branch: 'allow' | 'revise' | 'block' | 'escalate' | 'error';
  latency_ms: number;
}

/**
 * Run the SDK's check + dispatch the appropriate callback. Returns the
 * string the caller should actually send to the customer.
 *
 * Verdicts map 1:1 to callbacks:
 *
 *   allow    → onAllow (default: return draft as-is)
 *   rewrite  → onRevise (default: return decision.safe_output ?? draft)
 *   block    → onBlock (required)
 *   escalate → onEscalate (required)
 *
 * Transport / decode / retry-exhausted errors go to `onError`. Default
 * is **fail-open** — return the original draft. Pass an explicit
 * `onError` if your domain prefers fail-closed.
 */
export async function guard(opts: GuardOptions): Promise<string> {
  const start = performance.now();
  const req: CheckRequest = {
    agent_id: opts.agentId,
    channel: opts.channel ?? 'chat',
    input: opts.input,
    proposed_output: opts.draft,
    domain: opts.domain ?? null,
    policies: [],
    // Server's `context` is `Record<string, unknown> | null`; null
    // matches the wire shape when no context is supplied.
    context: (opts.context ?? null) as unknown as Record<string, unknown>,
    trace_id: opts.traceId ?? null,
  };

  let decision: Decision;
  try {
    decision = await opts.client.check(req, opts.signal);
  } catch (e) {
    if (!(e instanceof SdkError)) throw e;
    const fallback = opts.onError
      ? await opts.onError(e, opts.draft)
      : opts.draft; // fail-open default
    opts.log?.({
      trace_id: opts.traceId ?? '',
      // Wire shape doesn't have an "error" verdict; we synthesise the
      // log line for observability without lying about the wire.
      verdict: 'allow',
      branch: 'error',
      latency_ms: Math.round(performance.now() - start),
    });
    return fallback;
  }

  const result = await dispatch(opts, decision);
  opts.log?.({
    trace_id: decision.trace_id,
    verdict: decision.verdict,
    branch: branchFor(decision.verdict),
    latency_ms: Math.round(performance.now() - start),
  });
  return result;
}

async function dispatch(opts: GuardOptions, decision: Decision): Promise<string> {
  switch (decision.verdict) {
    case 'allow':
      return opts.onAllow ? await opts.onAllow(opts.draft, decision) : opts.draft;
    case 'rewrite':
      return opts.onRevise
        ? await opts.onRevise(decision.safe_output ?? null, opts.draft, decision)
        : (decision.safe_output ?? opts.draft);
    case 'block':
      return await opts.onBlock(decision);
    case 'escalate':
      return await opts.onEscalate(decision);
  }
}

function branchFor(v: Decision['verdict']): GuardLogEvent['branch'] {
  if (v === 'rewrite') return 'revise';
  return v;
}
