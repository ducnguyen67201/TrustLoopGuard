// `guardAgent()` and `guard()` — agent/tool and output-boundary helpers.
//
// Most integrations should decorate the agent once at startup. The returned
// object keeps the same interface, so existing reply and local-tool call sites
// stay unchanged:
//
//   const agent = guardAgent(createAgent(), { agentId: 'acme-support-v3' });
//   const reply = await agent.reply(userMessage);
//   await sendToCustomer(reply);
//
// The lower-level guard form remains available for custom client ownership:
//
//   const reply = await guard({
//     client,
//     input,
//     draft,
//     context,
//     agentId: 'acme-support-v3',
//     onBlock:    () => cannedSafeReply,
//     onRequireApproval: () => { humanQueue.push(...); return holdMessage; },
//     onDefer: () => 'Try again when required evidence is available.',
//   });
//   await sendToCustomer(reply);
//
// The handler stays the source of truth on what to do per effect.
// `guard` just makes the dispatch ergonomic and applies a fail-open
// default for transport errors so an outage on our side doesn't take
// down the agent.
//
// Factory guards support three presets:
//   strict                -> treat transform effects as denied output
//   rewrite               -> use transformed output, deny when none exists
//   rewrite_or_regenerate -> use transformed output, otherwise regenerate and check again

import {
  Client,
  createAutomaticRunController,
  type AutomaticRunController,
  type AutomaticRunTerminalStatus,
  type AutomaticRunWarning,
  type ClientOptions,
  type WithRunOptions,
} from './client.js';
import type { Channel } from './generated/Channel.js';
import type { CreateRunEventRequest } from './generated/CreateRunEventRequest.js';
import type { AuthorizationDecision as Decision } from './generated/AuthorizationDecision.js';
import type { GuardEvent } from './generated/GuardEvent.js';
import { SdkError } from './errors.js';
import { decorateAgentTools, type GuardToolDiscoveryOptions } from './tool-discovery.js';

const DEFAULT_BLOCK_MESSAGE = "I can't help with that request.";
const DEFAULT_REQUIRE_APPROVAL_MESSAGE = 'A human teammate should review this before we continue.';
const DEFAULT_DEFER_MESSAGE =
  'Required evidence or system state is unavailable. Please try again later.';

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
   * Called when the effect is `permit`. Default: return the original
   * draft unchanged. Override only if you want to log the permit path
   * or strip a draft suffix etc.
   */
  onAllow?: (draft: string, decision: Decision) => string | Promise<string>;

  /**
   * Called when the effect is `transform`. Default: return
   * `decision.transformed_value ?? draft`. Override to post-process or
   * substitute your own safe output.
   */
  onRevise?: (
    revised: string | null,
    draft: string,
    decision: Decision,
  ) => string | Promise<string>;

  /**
   * Called when the effect is `deny`. **Required** — there is no
   * sensible automatic answer. Return the canned safe message your
   * brand wants the customer to see (or throw to abort the send).
   */
  onBlock: (decision: Decision) => string | Promise<string>;

  /**
   * Called when the effect is `require_approval`. **Required** — typically
   * points the caller at the common authorization queue and returns a holding message.
   */
  onRequireApproval: (decision: Decision) => string | Promise<string>;

  /**
   * Called when the effect is `defer`. **Required** — approval cannot satisfy
   * this branch; wait for evidence or system availability before retrying.
   */
  onDefer: (decision: Decision) => string | Promise<string>;

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

  /** @deprecated Event submission assigns trace ids server-side. */
  traceId?: string;

  /** Optional run id used to group this check in the dashboard. */
  runId?: string;

  /** Optional existing run event id to attach to this check. Requires runId. */
  runEventId?: string;

  /** @deprecated Create run events explicitly and pass runEventId. */
  runEvent?: CreateRunEventRequest;

  /**
   * Logger hook. If provided, gets one structured event per `guard`
   * invocation: { trace_id, effect, branch, latency_ms }. Useful for
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

  /** Default approval-required branch. Omit for the SDK holding message. */
  onRequireApproval?: DecisionHandler;

  /** Default unresolved-evidence branch. Omit for the SDK retry-later message. */
  onDefer?: DecisionHandler;

  /** Transport failure branch. Omit for fail-open. */
  onError?: ErrorHandler;

  /**
   * Output mode:
   * - strict: treat transform effects as denied output
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

export interface GuardAgentOptions extends GuardFactoryOptions {
  /** Automatic discovery and guarding for supported local tool registries. */
  tools?: GuardToolDiscoveryOptions;

  /**
   * Automatic Run grouping. Defaults to one chat_session Run per reply.
   * Supply a session lifecycle to reuse one Run, or false for ungrouped traces.
   */
  run?: false | GuardAgentRunOptions;
}

export type GuardAgentRunWarning = AutomaticRunWarning;

export interface GuardAgentReplyRunOptions {
  /** Reply-scoped Runs are the safe fallback when no framework session exists. */
  scope?: 'reply';

  /** Run kind for each guarded reply. Defaults to `chat_session`. */
  kind?: WithRunOptions['kind'];

  /** Additive metadata stored on each automatically created Run. */
  metadata?: WithRunOptions['metadata'];

  /** Best-effort notification when automatic Run persistence fails. */
  onLifecycleWarning?: (warning: GuardAgentRunWarning) => void;
}

export interface GuardAgentSessionRunOptions {
  /** Keep one automatic Run for the registered framework session lifecycle. */
  scope: 'session';

  /** Stable upstream session correlation id, resolved lazily on first activity. */
  externalId: string | (() => string | Promise<string>);

  /** Register the framework's deterministic end boundary. */
  registerEnd: (
    finish: (status: AutomaticRunTerminalStatus) => Promise<void>,
  ) => void | (() => void);

  /** Run kind for the session. Defaults to chat_session. */
  kind?: WithRunOptions['kind'];

  /** Additive metadata stored on the session Run. */
  metadata?: WithRunOptions['metadata'];

  /** Best-effort notification when automatic Run persistence fails. */
  onLifecycleWarning?: (warning: GuardAgentRunWarning) => void;
}

export type GuardAgentRunOptions = GuardAgentReplyRunOptions | GuardAgentSessionRunOptions;

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
  /** @deprecated Create run events explicitly and pass runEventId. */
  runEvent?: CreateRunEventRequest;
  onBlock?: DecisionHandler;
  onRequireApproval?: DecisionHandler;
  onDefer?: DecisionHandler;
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

export interface GuardWrapOptions<Args extends unknown[]> {
  /**
   * Select the user input from the wrapped function arguments.
   * By default, `wrap()` uses the first argument and requires it to be a string.
   */
  input?: (...args: Args) => string;
}

export interface ReplyAgent<Args extends unknown[] = unknown[]> {
  reply(message: string, ...args: Args): Promise<string>;
}

export interface OutputGuard {
  (opts: GuardCallOptions): Promise<string>;
  /**
   * Wrap an agent function so its returned string is checked before it reaches
   * the caller. The first argument is treated as the user input by default.
   */
  wrap<Args extends unknown[]>(
    fn: (...args: Args) => string | Promise<string>,
    opts?: GuardWrapOptions<Args>,
  ): (...args: Args) => Promise<string>;
  /**
   * Streaming form: consume a token/chunk stream, buffer it in full, then run
   * the same guard as the non-streaming call. Returns the guarded string the
   * caller should deliver.
   */
  stream(opts: GuardStreamCallOptions): Promise<string>;
}

export interface GuardLogEvent {
  trace_id: string;
  effect: Decision['effect'];
  /** Which callback we ended up calling. */
  branch: 'permit' | 'revise' | 'deny' | 'require_approval' | 'defer' | 'error';
  latency_ms: number;
}

/**
 * Submit an output `GuardEvent` and dispatch the appropriate callback. Returns the
 * string the caller should actually send to the customer.
 *
 * Effects map 1:1 to callbacks:
 *
 *   permit           → onAllow (default: return draft as-is)
 *   transform        → onRevise (default: return decision.transformed_value ?? draft)
 *   deny             → onBlock (required)
 *   require_approval → onRequireApproval (required)
 *   defer            → onDefer (required)
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

/**
 * Decorate an agent object at its output and local-tool boundaries.
 *
 * The returned object keeps the agent's public interface. When `reply()` is
 * present, its final string passes through TrustLoopGuard before it reaches the
 * caller. Supported local tool registries are discovered and their `execute()`
 * methods are authorized before side effects run.
 */
export function guardAgent<Agent extends object>(agent: Agent, opts: GuardAgentOptions): Agent {
  const client = opts.client ?? new Client(clientOptions(opts));
  let automaticRun: AutomaticRunController | undefined;
  let toolAutomaticRun: AutomaticRunController | undefined;
  if (opts.run !== false) {
    const metadata = { ...(opts.run?.metadata ?? {}), integration: 'guardAgent' };
    if (opts.run?.scope === 'session') {
      automaticRun = createAutomaticRunController(client, {
        agentId: opts.agentId,
        scope: 'session',
        kind: opts.run.kind ?? 'chat_session',
        metadata,
        externalId: opts.run.externalId,
        registerEnd: opts.run.registerEnd,
        ...(opts.run.onLifecycleWarning === undefined
          ? {}
          : { onLifecycleWarning: opts.run.onLifecycleWarning }),
      });
      toolAutomaticRun = automaticRun;
    } else {
      automaticRun = createAutomaticRunController(client, {
        agentId: opts.agentId,
        scope: 'reply',
        kind: opts.run?.kind ?? 'chat_session',
        metadata,
        ...(opts.run?.onLifecycleWarning === undefined
          ? {}
          : { onLifecycleWarning: opts.run.onLifecycleWarning }),
      });
    }
  }
  const toolOptions = {
    agentId: opts.agentId,
    client,
    ...(opts.context !== undefined ? { context: opts.context } : {}),
    ...(opts.tools !== undefined ? { tools: opts.tools } : {}),
    ...(toolAutomaticRun === undefined ? {} : { automaticRun: toolAutomaticRun }),
  };
  decorateAgentTools(agent, toolOptions);

  const reply = Reflect.get(agent, 'reply', agent);
  const guardedOutputReply =
    typeof reply === 'function'
      ? createOutputGuard({ ...opts, client }).wrap(reply.bind(agent))
      : undefined;
  const guardedReply =
    guardedOutputReply === undefined || automaticRun === undefined
      ? guardedOutputReply
      : (...args: Parameters<typeof guardedOutputReply>) =>
          automaticRun.run(() => guardedOutputReply(...args));
  const boundMethods = new WeakMap<object, unknown>();

  return new Proxy(agent, {
    get(target, property) {
      if (property === 'reply' && guardedReply !== undefined) return guardedReply;

      const value = Reflect.get(target, property, target);
      if (typeof value !== 'function') return value;

      const cached = boundMethods.get(value);
      if (cached !== undefined) return cached;

      const bound = value.bind(target);
      boundMethods.set(value, bound);
      return bound;
    },
    set(target, property, value) {
      return Reflect.set(target, property, value, target);
    },
  });
}

async function guardOnce(opts: GuardOptions): Promise<string> {
  const start = performance.now();
  const event = outputEvent(opts);

  let decision: Decision;
  try {
    decision = await opts.client.submitEvent(event, opts.signal);
  } catch (e) {
    if (!(e instanceof SdkError)) throw e;
    const fallback = opts.onError ? await opts.onError(e, opts.draft) : opts.draft; // fail-open default
    opts.log?.({
      trace_id: opts.traceId ?? '',
      // Wire shape doesn't have an "error" effect; we synthesise the
      // log line for observability without lying about the wire.
      effect: 'permit',
      branch: 'error',
      latency_ms: Math.round(performance.now() - start),
    });
    return fallback;
  }

  const result = await dispatch(opts, decision);
  opts.log?.({
    trace_id: decision.trace_id,
    effect: decision.effect,
    branch: branchFor(decision.effect),
    latency_ms: Math.round(performance.now() - start),
  });
  return result;
}

function outputEvent(opts: GuardOptions): GuardEvent {
  const context: Record<string, unknown> = {
    ...(opts.context ?? {}),
    channel: opts.channel ?? 'chat',
    domain: opts.domain ?? 'customer_support',
  };
  const event: GuardEvent = {
    kind: 'output.proposed',
    principal: {
      workspace_id: '',
      environment_id: '',
      agent_id: opts.agentId,
    },
    action: {
      operation: 'output',
      parameters: { text: opts.draft },
      side_effect: 'none',
    },
    sources: [
      {
        id: 'input',
        origin: 'user',
        labels: {
          trust: 'unknown',
          confidentiality: 'unknown',
          integrity: 'unknown',
        },
      },
    ],
    provenance: { text: ['input'] },
    context,
  };
  addDefined(event.principal, 'run_id', opts.runId);
  addDefined(event.principal, 'run_event_id', opts.runEventId);
  return event;
}

function createOutputGuard(opts: GuardFactoryOptions): OutputGuard {
  const client = opts.client ?? new Client(clientOptions(opts));

  const guardFn = async (call: GuardCallOptions) => {
    const mode = call.mode ?? opts.mode ?? 'rewrite';
    const regenerate = call.regenerate ?? opts.regenerate;
    const maxRegenerations = call.maxRegenerations ?? opts.maxRegenerations ?? 1;
    const onBlock = decisionHandler(call.onBlock ?? opts.onBlock, DEFAULT_BLOCK_MESSAGE);
    const onRequireApproval = decisionHandler(
      call.onRequireApproval ?? opts.onRequireApproval,
      DEFAULT_REQUIRE_APPROVAL_MESSAGE,
    );
    const onDefer = decisionHandler(call.onDefer ?? opts.onDefer, DEFAULT_DEFER_MESSAGE);
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
          safeOutput:
            typeof decision.transformed_value === 'string' ? decision.transformed_value : null,
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
        onRequireApproval,
        onDefer,
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
  outputGuard.wrap = <Args extends unknown[]>(
    fn: (...args: Args) => string | Promise<string>,
    wrapOptions?: GuardWrapOptions<Args>,
  ) => {
    return async (...args: Args): Promise<string> => {
      const input = wrapOptions?.input ? wrapOptions.input(...args) : args[0];
      if (typeof input !== 'string') {
        throw new TypeError(
          'guard.wrap() input must be a string; pass { input: (...args) => string } ' +
            'for structured arguments',
        );
      }

      const draft = await fn(...args);
      if (typeof draft !== 'string') {
        throw new TypeError('guard.wrap() wrapped function must return a string');
      }

      const call: GuardCallOptions = { input, draft };
      if (opts.failClosed === undefined && opts.onError === undefined) {
        call.onError = DEFAULT_BLOCK_MESSAGE;
      }
      return await guardFn(call);
    };
  };
  return outputGuard;
}

async function dispatch(opts: GuardOptions, decision: Decision): Promise<string> {
  switch (decision.effect) {
    case 'permit':
      return opts.onAllow ? await opts.onAllow(opts.draft, decision) : opts.draft;
    case 'transform': {
      const transformed =
        typeof decision.transformed_value === 'string' ? decision.transformed_value : null;
      return opts.onRevise
        ? await opts.onRevise(transformed, opts.draft, decision)
        : (transformed ?? opts.draft);
    }
    case 'deny':
      return await opts.onBlock(decision);
    case 'require_approval':
      return await opts.onRequireApproval(decision);
    case 'defer':
      return await opts.onDefer(decision);
  }
}

function branchFor(v: Decision['effect']): GuardLogEvent['branch'] {
  if (v === 'transform') return 'revise';
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
      env('TLG_URL', 'TL_SERVER_URL', 'TRUSTLOOPGUARD_URL', 'TRUSTLOOP_URL') ??
      'http://127.0.0.1:8080',
  };
  addDefined(
    clientOpts,
    'apiKey',
    opts.apiKey ?? env('TLG_API_KEY', 'TL_API_KEY', 'TRUSTLOOPGUARD_API_KEY', 'TRUSTLOOP_API_KEY'),
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
