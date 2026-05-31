// `guard()` — output-boundary helper.
//
// Most integrations should create one guardrail at startup and call it before
// delivering an agent draft:
//
//   const guardrail = guard({ agentId: 'acme-support-v3' });
//   const reply = await guardrail({ input: userMessage, draft: agentDraft });
//   await sendToCustomer(reply);
//
// The lower-level form remains available for custom client ownership:
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
//
// Factory guards support three presets:
//   strict                -> treat rewrite verdicts as blocked output
//   rewrite               -> use safeOutput, block when no safeOutput exists
//   rewrite_or_regenerate -> use safeOutput, otherwise regenerate and check again

import { Client, type ClientOptions } from './client';
import type { Channel } from './generated/Channel';
import type { CheckRequest } from './generated/CheckRequest';
import type { CreateRunEventRequest } from './generated/CreateRunEventRequest';
import type { Decision } from './generated/Decision';
import { SdkError } from './errors';

const DEFAULT_BLOCK_MESSAGE = "I can't help with that request.";
const DEFAULT_ESCALATE_MESSAGE = 'A human teammate should review this before we continue.';

type DecisionHandler = string | ((decision: Decision) => string | Promise<string>);
type ErrorHandler = string | ((err: SdkError, draft: string) => string | Promise<string>);
export const GuardMode = {
  Strict: 'strict',
  Rewrite: 'rewrite',
  RewriteOrRegenerate: 'rewrite_or_regenerate',
} as const;
export type GuardMode = (typeof GuardMode)[keyof typeof GuardMode];

export interface RegenerateFeedback {
  /** What the user said. */
  input: string;

  /** The draft that failed the guard check. */
  draft: string;

  /** Full TrustLoopGuard decision for the failed draft. */
  decision: Decision;

  /** Human-readable reason returned by TrustLoopGuard. */
  reason: string;

  /** Safe output returned by TrustLoopGuard, when available. */
  safeOutput: string | null;

  /** 1-based regeneration attempt number. */
  attempt: number;

  /** Maximum allowed regeneration attempts. */
  maxAttempts: number;
}

type RegenerateHandler = (feedback: RegenerateFeedback) => string | Promise<string>;

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
  onRevise?: (
    revised: string | null,
    draft: string,
    decision: Decision,
  ) => string | Promise<string>;

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

  /** Optional run id used to group this check in the dashboard. */
  runId?: string;

  /** Optional existing run event id to attach to this check. Requires runId. */
  runEventId?: string;

  /** Optional inline run event to create and attach to this check. Requires runId. */
  runEvent?: CreateRunEventRequest;

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

export interface GuardFactoryOptions {
  /** Required: the registered agent profile id. */
  agentId: string;

  /** Existing client. Pass this when you want to own transport lifecycle/config. */
  client?: Client;

  /** TrustLoopGuard server URL. Defaults to env or localhost. */
  baseUrl?: string;

  /** Bearer token. Defaults to env when available. */
  apiKey?: string;

  /** Retry policy forwarded when the factory owns the Client. */
  retry?: ClientOptions['retry'];

  /** Fetch implementation forwarded when the factory owns the Client. */
  fetchImpl?: ClientOptions['fetchImpl'];

  /** Retry logger forwarded when the factory owns the Client. */
  onRetry?: ClientOptions['onRetry'];

  /** Conversation channel — drives latency budget on the server. */
  channel?: Channel;

  /** Optional override for `domain`. */
  domain?: string;

  /** Structured context merged into every call. */
  context?: Record<string, unknown>;

  /** Default block branch. Omit for the SDK safe message. */
  onBlock?: DecisionHandler;

  /** Default escalation branch. Omit for the SDK safe message. */
  onEscalate?: DecisionHandler;

  /** Transport failure branch. Omit for fail-open. */
  onError?: ErrorHandler;

  /**
   * Output mode:
   * - strict: treat rewrite verdicts as blocked output
   * - rewrite: use safeOutput, block when no safeOutput exists
   * - rewrite_or_regenerate: use safeOutput, otherwise ask the model to try again
   */
  mode?: GuardMode;

  /** Called by rewrite_or_regenerate when TrustLoopGuard has no safeOutput. */
  regenerate?: RegenerateHandler;

  /** Hard cap for model regeneration loops. Defaults to 1. */
  maxRegenerations?: number;

  /** Return the default block message on transport errors when no onError is set. */
  failClosed?: boolean;

  /** Logger hook for every guard invocation. */
  log?: (event: GuardLogEvent) => void;
}

export interface GuardCallOptions {
  /** What the user said. */
  input: string;

  /** What the agent wants to send. */
  draft: string;

  channel?: Channel;
  domain?: string;
  context?: Record<string, unknown>;
  traceId?: string;
  runId?: string;
  runEventId?: string;
  runEvent?: CreateRunEventRequest;
  onBlock?: DecisionHandler;
  onEscalate?: DecisionHandler;
  onError?: ErrorHandler;
  mode?: GuardMode;
  regenerate?: RegenerateHandler;
  maxRegenerations?: number;
  log?: (event: GuardLogEvent) => void;
  signal?: AbortSignal;
}

/** Streaming form of {@link GuardCallOptions}: the agent's output arrives as a
 *  chunk/token stream instead of a finished string. */
export interface GuardStreamCallOptions extends Omit<GuardCallOptions, 'draft'> {
  /**
   * The agent's output as an async stream of chunks (e.g. an LLM token
   * stream). The guard buffers the full stream, then guards the complete
   * output — it never returns unguarded chunks, mirroring the gateway's
   * buffered-then-emit model.
   */
  draft: AsyncIterable<string>;
}

export interface OutputGuard {
  (opts: GuardCallOptions): Promise<string>;
  /**
   * Streaming form: consume a token/chunk stream, buffer it in full, then run
   * the same guard as the non-streaming call. Returns the guarded string the
   * caller should deliver.
   */
  stream(opts: GuardStreamCallOptions): Promise<string>;
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
export function guard(opts: GuardFactoryOptions): OutputGuard;
export function guard(opts: GuardOptions): Promise<string>;
export function guard(opts: GuardFactoryOptions | GuardOptions): OutputGuard | Promise<string> {
  if ('input' in opts && 'draft' in opts) {
    return guardOnce(opts as GuardOptions);
  }
  return createOutputGuard(opts);
}

async function guardOnce(opts: GuardOptions): Promise<string> {
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
  addDefined(req, 'run_id', opts.runId);
  addDefined(req, 'run_event_id', opts.runEventId);
  addDefined(req, 'run_event', opts.runEvent);

  let decision: Decision;
  try {
    decision = await opts.client.check(req, opts.signal);
  } catch (e) {
    if (!(e instanceof SdkError)) throw e;
    const fallback = opts.onError ? await opts.onError(e, opts.draft) : opts.draft; // fail-open default
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

function createOutputGuard(opts: GuardFactoryOptions): OutputGuard {
  const client = opts.client ?? new Client(clientOptions(opts));

  const guardFn = async (call: GuardCallOptions) => {
    const mode = call.mode ?? opts.mode ?? 'rewrite';
    const regenerate = call.regenerate ?? opts.regenerate;
    const maxRegenerations = call.maxRegenerations ?? opts.maxRegenerations ?? 1;
    const onBlock = decisionHandler(call.onBlock ?? opts.onBlock, DEFAULT_BLOCK_MESSAGE);
    const onEscalate = decisionHandler(
      call.onEscalate ?? opts.onEscalate,
      DEFAULT_ESCALATE_MESSAGE,
    );
    const onError = errorHandler(
      call.onError ?? opts.onError,
      opts.failClosed === true ? DEFAULT_BLOCK_MESSAGE : undefined,
    );

    const runAttempt = async (
      currentDraft: string,
      completedRegenerations: number,
    ): Promise<string> => {
      const onRevise = async (
        revised: string | null,
        checkedDraft: string,
        decision: Decision,
      ): Promise<string> => {
        if (mode === 'strict') return await onBlock(decision);
        if (revised !== null) return revised;
        if (
          mode !== 'rewrite_or_regenerate' ||
          regenerate === undefined ||
          completedRegenerations >= maxRegenerations
        ) {
          return await onBlock(decision);
        }

        const nextAttempt = completedRegenerations + 1;
        const nextDraft = await regenerate({
          input: call.input,
          draft: checkedDraft,
          decision,
          reason: decision.reason,
          safeOutput: decision.safe_output ?? null,
          attempt: nextAttempt,
          maxAttempts: maxRegenerations,
        });
        return await runAttempt(nextDraft, nextAttempt);
      };

      const guardOpts: GuardOptions = {
        client,
        agentId: opts.agentId,
        input: call.input,
        draft: currentDraft,
        context: { ...(opts.context ?? {}), ...(call.context ?? {}) },
        onBlock,
        onEscalate,
        onRevise,
      };
      addDefined(guardOpts, 'channel', call.channel ?? opts.channel);
      addDefined(guardOpts, 'domain', call.domain ?? opts.domain);
      addDefined(guardOpts, 'traceId', call.traceId);
      addDefined(guardOpts, 'runId', call.runId);
      addDefined(guardOpts, 'runEventId', call.runEventId);
      addDefined(guardOpts, 'runEvent', call.runEvent);
      addDefined(guardOpts, 'onError', onError);
      addDefined(guardOpts, 'log', call.log ?? opts.log);
      addDefined(guardOpts, 'signal', call.signal);

      return await guardOnce(guardOpts);
    };

    return await runAttempt(call.draft, 0);
  };

  const outputGuard = guardFn as OutputGuard;
  outputGuard.stream = async (call: GuardStreamCallOptions): Promise<string> => {
    let draft = '';
    for await (const chunk of call.draft) {
      draft += chunk;
    }
    return guardFn({ ...call, draft });
  };
  return outputGuard;
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

function decisionHandler(
  handler: DecisionHandler | undefined,
  defaultMessage: string,
): (decision: Decision) => string | Promise<string> {
  return async (decision) => {
    if (handler === undefined) return defaultMessage;
    if (typeof handler === 'string') return handler;
    return await handler(decision);
  };
}

function errorHandler(
  handler: ErrorHandler | undefined,
  defaultMessage: string | undefined,
): ((err: SdkError, draft: string) => string | Promise<string>) | undefined {
  if (handler === undefined && defaultMessage === undefined) return undefined;
  return async (err, draft) => {
    if (handler === undefined) return defaultMessage ?? draft;
    if (typeof handler === 'string') return handler;
    return await handler(err, draft);
  };
}

function env(...names: string[]): string | undefined {
  const proc = globalThis as typeof globalThis & {
    process?: { env?: Record<string, string | undefined> };
  };
  for (const name of names) {
    const value = proc.process?.env?.[name];
    if (value !== undefined && value.length > 0) return value;
  }
  return undefined;
}

function clientOptions(opts: GuardFactoryOptions): ClientOptions {
  const clientOpts: ClientOptions = {
    baseUrl:
      opts.baseUrl ??
      env('TL_SERVER_URL', 'TRUSTLOOPGUARD_URL', 'TRUSTLOOP_URL') ??
      'http://127.0.0.1:8080',
  };
  addDefined(
    clientOpts,
    'apiKey',
    opts.apiKey ?? env('TL_API_KEY', 'TRUSTLOOPGUARD_API_KEY', 'TRUSTLOOP_API_KEY'),
  );
  addDefined(clientOpts, 'retry', opts.retry);
  addDefined(clientOpts, 'fetchImpl', opts.fetchImpl);
  addDefined(clientOpts, 'onRetry', opts.onRetry);
  return clientOpts;
}

function addDefined<T extends object, K extends keyof T>(
  target: T,
  key: K,
  value: T[K] | undefined,
): void {
  if (value !== undefined) {
    target[key] = value;
  }
}
