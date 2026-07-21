// Thin HTTP client over the public GuardEvent runtime contract.
//
// Retry policy mirrors tl-sdk-rust and the Python SDK — same defaults,
// same `nextDelay` semantics. Voice callers should pass
// `{ ...DEFAULT_RETRY, maxAttempts: 1 }` to opt out.

import type { AuthorizationDecision } from './generated/AuthorizationDecision.js';
import type { AuthorizationClaim } from './generated/AuthorizationClaim.js';
import type { AuthorizationApproval } from './generated/AuthorizationApproval.js';
import type { AuthorizationApprovalListResponse } from './generated/AuthorizationApprovalListResponse.js';
import type { AuthorizationGrant } from './generated/AuthorizationGrant.js';
import type { AuthorizationGrantListResponse } from './generated/AuthorizationGrantListResponse.js';
import type { AuthorizationReceipt } from './generated/AuthorizationReceipt.js';
import type { AuthorizationReceiptListResponse } from './generated/AuthorizationReceiptListResponse.js';
import type { CompleteAuthorizationLeaseRequest } from './generated/CompleteAuthorizationLeaseRequest.js';
import type { CreateAuthorizationGrantRequest } from './generated/CreateAuthorizationGrantRequest.js';
import type { DecideAuthorizationApprovalRequest } from './generated/DecideAuthorizationApprovalRequest.js';
import type { DecideAuthorizationApprovalResponse } from './generated/DecideAuthorizationApprovalResponse.js';
import type { GuardEvent } from './generated/GuardEvent.js';
import type { EventKind } from './generated/EventKind.js';
import type { AgentListResponse } from './generated/AgentListResponse.js';
import type { AgentProfile } from './generated/AgentProfile.js';
import type { ApiKeyBatchRevokeResponse } from './generated/ApiKeyBatchRevokeResponse.js';
import type { GuardrailGenerateResponse } from './generated/GuardrailGenerateResponse.js';
import type { GuardrailListResponse } from './generated/GuardrailListResponse.js';
import type { RedteamPlanRequest } from './generated/RedteamPlanRequest.js';
import type { RedteamPlanResponse } from './generated/RedteamPlanResponse.js';
import type { RedteamPlanListResponse } from './generated/RedteamPlanListResponse.js';
import type { PolicyDocument } from './generated/PolicyDocument.js';
import type { PolicyBatchSetEnabledResponse } from './generated/PolicyBatchSetEnabledResponse.js';
import type { PolicyDraftResponse } from './generated/PolicyDraftResponse.js';
import type { PolicyFamily } from './generated/PolicyFamily.js';
import type { PolicyListResponse } from './generated/PolicyListResponse.js';
import type { PolicyValidateResponse } from './generated/PolicyValidateResponse.js';
import type { AgenticPaymentAuthorizationResponse } from './generated/AgenticPaymentAuthorizationResponse.js';
import type { AgenticPaymentAuthorizeRequest } from './generated/AgenticPaymentAuthorizeRequest.js';
import type { AgenticPaymentCommitRequest } from './generated/AgenticPaymentCommitRequest.js';
import type { AgenticPaymentRecord } from './generated/AgenticPaymentRecord.js';
import type { AgenticPaymentRollbackRequest } from './generated/AgenticPaymentRollbackRequest.js';
import type { CreateFinancialActionRequest } from './generated/CreateFinancialActionRequest.js';
import type { ExecuteFinancialActionRequest } from './generated/ExecuteFinancialActionRequest.js';
import type { CreateFinancialPolicyRequest } from './generated/CreateFinancialPolicyRequest.js';
import type { CounterpartyRef } from './generated/CounterpartyRef.js';
import type { EvidenceRef } from './generated/EvidenceRef.js';
import type { FinancialActionKind } from './generated/FinancialActionKind.js';
import type { FinancialActionListResponse } from './generated/FinancialActionListResponse.js';
import type { FinancialActionOutcome } from './generated/FinancialActionOutcome.js';
import type { FinancialRail } from './generated/FinancialRail.js';
import type { MoneyAmount } from './generated/MoneyAmount.js';
import type { FinancialOutcomeListResponse } from './generated/FinancialOutcomeListResponse.js';
import type { FinancialPolicyListResponse } from './generated/FinancialPolicyListResponse.js';
import type { FinancialPolicyRecord } from './generated/FinancialPolicyRecord.js';
import type { FinancialReceipt } from './generated/FinancialReceipt.js';
import type { CreateRunEventRequest } from './generated/CreateRunEventRequest.js';
import type { CreateRunRequest } from './generated/CreateRunRequest.js';
import type { RunDetail } from './generated/RunDetail.js';
import type { RunEventListResponse } from './generated/RunEventListResponse.js';
import type { RunEventSummary } from './generated/RunEventSummary.js';
import type { RunKind } from './generated/RunKind.js';
import type { RunListResponse } from './generated/RunListResponse.js';
import type { RunStatus } from './generated/RunStatus.js';
import type { RunSummary } from './generated/RunSummary.js';
import type { FinancialActionRecord } from './generated/FinancialActionRecord.js';
import type { ProvenanceMap } from './generated/ProvenanceMap.js';
import type { SideEffectClass } from './generated/SideEffectClass.js';
import type { ShellActionParameters } from './generated/ShellActionParameters.js';
import type { ShellLanguage } from './generated/ShellLanguage.js';
import type { Source } from './generated/Source.js';
import type { TraceListResponse } from './generated/TraceListResponse.js';
import type { ToolMetadataEntry } from './generated/ToolMetadataEntry.js';
import type { ToolMetadataListResponse } from './generated/ToolMetadataListResponse.js';
import type { ToolIdentity } from './generated/ToolIdentity.js';
import type { UpdateRunRequest } from './generated/UpdateRunRequest.js';
import type { UpsertToolMetadataRequest } from './generated/UpsertToolMetadataRequest.js';
import { Decode, SdkError, Transport, fromResponse, parseRetryAfter } from './errors.js';
import { DEFAULT_RETRY, type RetryConfig, nextDelay } from './retry.js';

export interface ClientOptions {
  baseUrl: string;
  apiKey?: string;
  fetchImpl?: typeof fetch;
  retry?: RetryConfig;
  /**
   * Hook invoked once per retry decision. Useful for surfacing retry
   * activity in logs / OpenTelemetry without forcing a logger
   * dependency on the SDK.
   */
  onRetry?: (info: { attempt: number; delayS: number; error: SdkError }) => void;
}

interface ActiveRunContext {
  runId?: string;
  runEventId?: string;
}

interface RunContextStore {
  isolated: boolean;
  getStore(): ActiveRunContext | undefined;
  run<T>(store: ActiveRunContext, callback: () => Promise<T>): Promise<T>;
}

function browserRunContext(): RunContextStore {
  let current: ActiveRunContext | undefined;
  return {
    isolated: false,
    getStore: () => current,
    async run(store, callback) {
      const previous = current;
      current = store;
      try {
        return await callback();
      } finally {
        current = previous;
      }
    },
  };
}

let runContextStore: Promise<RunContextStore> | undefined;

async function runContext(): Promise<RunContextStore> {
  runContextStore ??= (async () => {
    const nodeVersion = (globalThis as { process?: { versions?: { node?: string } } }).process
      ?.versions?.node;
    if (nodeVersion) {
      try {
        const asyncHooks = 'node:async_hooks';
        const mod = (await import(asyncHooks)) as {
          AsyncLocalStorage: new () => Omit<RunContextStore, 'isolated'>;
        };
        const storage = new mod.AsyncLocalStorage();
        return {
          isolated: true,
          getStore: () => storage.getStore(),
          run: (store, callback) => storage.run(store, callback),
        };
      } catch {
        // Browser/edge bundles can still execute the fallback.
      }
    }
    return browserRunContext();
  })();
  return runContextStore;
}

export interface WithRunOptions {
  agentId: string;
  externalId?: string;
  kind?: RunKind;
  metadata?: Record<string, unknown>;
  inputSummary?: string;
  finishOnError?: boolean;
}

export interface ActiveRun {
  id: string;
  withEvent<T>(
    req: Omit<CreateRunEventRequest, 'metadata'> & { metadata?: Record<string, unknown> },
    fn: () => Promise<T>,
  ): Promise<T>;
  finish(status?: Extract<RunStatus, 'completed' | 'failed' | 'canceled'>): Promise<void>;
}

export type AutomaticRunTerminalStatus = Extract<RunStatus, 'completed' | 'failed' | 'canceled'>;

export interface AutomaticRunWarning {
  code: 'run_start_failed' | 'run_event_create_failed' | 'run_finish_failed';
  phase: 'start' | 'event' | 'finish';
  error: SdkError;
}

export interface AutomaticRunController {
  run<T>(fn: () => Promise<T>): Promise<T>;
  withEvent<T>(
    req: Omit<CreateRunEventRequest, 'metadata'> & { metadata?: Record<string, unknown> },
    fn: () => Promise<T>,
  ): Promise<T>;
}

export interface AutomaticRunOptions {
  agentId: string;
  scope: 'reply' | 'session';
  kind: RunKind;
  metadata: NonNullable<WithRunOptions['metadata']>;
  externalId?: string | (() => string | Promise<string>);
  registerEnd?: (
    finish: (status: AutomaticRunTerminalStatus) => Promise<void>,
  ) => void | (() => void);
  onLifecycleWarning?: (warning: AutomaticRunWarning) => void;
}

/**
 * Build one best-effort Run controller for a high-level agent decorator.
 * Existing explicit scopes always win. Reply scope is one-shot; session scope
 * keeps one lazily created Run until the registered framework lifecycle ends.
 */
export function createAutomaticRunController(
  client: Client,
  opts: AutomaticRunOptions,
): AutomaticRunController {
  if (opts.scope === 'reply') {
    return {
      run: <T>(fn: () => Promise<T>) => withAutomaticReplyRun(client, opts, fn),
      withEvent: <T>(
        req: Omit<CreateRunEventRequest, 'metadata'> & { metadata?: Record<string, unknown> },
        fn: () => Promise<T>,
      ) => withAutomaticRunEvent(client, opts, req, fn),
    };
  }
  if (opts.externalId === undefined || opts.registerEnd === undefined) {
    throw new TypeError('guardAgent session runs require externalId and registerEnd');
  }
  return new SessionAutomaticRunController(client, opts);
}

async function withAutomaticReplyRun<T>(
  client: Client,
  opts: AutomaticRunOptions,
  fn: () => Promise<T>,
): Promise<T> {
  const context = await runContext();
  // Raw turn capture must never use the shared browser fallback context: two
  // overlapping promises could otherwise attach one customer's text to
  // another Run. Guard enforcement still proceeds without automatic grouping.
  if (!context.isolated) return fn();
  if (context.getStore()?.runId) return fn();

  let summary: RunSummary;
  try {
    summary = await startAutomaticRun(client, opts);
  } catch (error) {
    if (error instanceof SdkError) {
      notifyAutomaticRunWarning(opts, 'start', error);
      return fn();
    }
    throw error;
  }

  return withRunId(summary.id, async () => {
    try {
      const result = await fn();
      try {
        await client.finishRun(summary.id, 'completed');
      } catch (error) {
        if (error instanceof SdkError) {
          notifyAutomaticRunWarning(opts, 'finish', error);
        } else {
          throw error;
        }
      }
      return result;
    } catch (error) {
      try {
        await client.finishRun(summary.id, 'failed');
      } catch (finishError) {
        if (finishError instanceof SdkError) {
          notifyAutomaticRunWarning(opts, 'finish', finishError);
        }
        // Preserve the guarded operation's error.
      }
      throw error;
    }
  });
}

class SessionAutomaticRunController implements AutomaticRunController {
  private startPromise: Promise<RunSummary> | undefined;
  private startFailed = false;
  private terminalStatus: AutomaticRunTerminalStatus | undefined;
  private finishPromise: Promise<void> | undefined;
  private unsubscribe: (() => void) | undefined;
  private activeBoundaries = 0;
  private idlePromise: Promise<void> | undefined;
  private resolveIdle: (() => void) | undefined;

  constructor(
    private readonly client: Client,
    private readonly opts: AutomaticRunOptions,
  ) {
    const registerEnd = opts.registerEnd;
    if (registerEnd === undefined) {
      throw new TypeError('guardAgent session runs require registerEnd');
    }
    const unsubscribe = registerEnd((status) => this.finish(status));
    if (typeof unsubscribe === 'function') this.unsubscribe = unsubscribe;
    if (this.terminalStatus !== undefined) this.detachLifecycle();
  }

  async run<T>(fn: () => Promise<T>): Promise<T> {
    const context = await runContext();
    if (!context.isolated) return fn();
    if (context.getStore()?.runId || this.terminalStatus !== undefined) return fn();

    this.beginBoundary();
    try {
      let summary: RunSummary;
      try {
        summary = await this.ensureRun();
      } catch (error) {
        if (error instanceof SdkError) return await fn();
        throw error;
      }
      return await withRunId(summary.id, fn);
    } finally {
      this.endBoundary();
    }
  }

  async withEvent<T>(
    req: Omit<CreateRunEventRequest, 'metadata'> & { metadata?: Record<string, unknown> },
    fn: () => Promise<T>,
  ): Promise<T> {
    return await withAutomaticRunEvent(this.client, this.opts, req, fn);
  }

  private ensureRun(): Promise<RunSummary> {
    this.startPromise ??= startAutomaticRun(this.client, this.opts).catch((error) => {
      if (error instanceof SdkError) {
        this.startFailed = true;
        notifyAutomaticRunWarning(this.opts, 'start', error);
      }
      throw error;
    });
    return this.startPromise;
  }

  private async finish(status: AutomaticRunTerminalStatus): Promise<void> {
    if (this.terminalStatus !== undefined) {
      return this.finishPromise ?? Promise.resolve();
    }
    this.terminalStatus = status;
    this.detachLifecycle();

    const startPromise = this.startPromise;
    if (startPromise === undefined) return;

    this.finishPromise = this.finishStartedRun(startPromise, status);
    return this.finishPromise;
  }

  private async finishStartedRun(
    startPromise: Promise<RunSummary>,
    status: AutomaticRunTerminalStatus,
  ): Promise<void> {
    let summary: RunSummary;
    try {
      summary = await startPromise;
    } catch (error) {
      if (error instanceof SdkError) return;
      throw error;
    }

    await this.waitForBoundaries();
    try {
      await this.client.finishRun(summary.id, status);
    } catch (error) {
      if (error instanceof SdkError) {
        notifyAutomaticRunWarning(this.opts, 'finish', error);
        return;
      }
      throw error;
    }
  }

  private beginBoundary(): void {
    if (this.activeBoundaries === 0) {
      this.idlePromise = new Promise<void>((resolve) => {
        this.resolveIdle = resolve;
      });
    }
    this.activeBoundaries += 1;
  }

  private endBoundary(): void {
    this.activeBoundaries -= 1;
    if (this.activeBoundaries !== 0) return;

    this.resolveIdle?.();
    this.resolveIdle = undefined;
    this.idlePromise = undefined;
    if (this.startFailed && this.terminalStatus === undefined) {
      this.startPromise = undefined;
      this.startFailed = false;
    }
  }

  private async waitForBoundaries(): Promise<void> {
    if (this.activeBoundaries > 0 && this.idlePromise !== undefined) {
      await this.idlePromise;
    }
  }

  private detachLifecycle(): void {
    const unsubscribe = this.unsubscribe;
    this.unsubscribe = undefined;
    if (unsubscribe === undefined) return;
    try {
      unsubscribe();
    } catch {
      // Framework listener cleanup is best-effort observability bookkeeping.
    }
  }
}

async function withAutomaticRunEvent<T>(
  client: Client,
  opts: AutomaticRunOptions,
  req: Omit<CreateRunEventRequest, 'metadata'> & { metadata?: Record<string, unknown> },
  fn: () => Promise<T>,
): Promise<T> {
  const context = await runContext();
  if (!context.isolated) return await fn();
  const current = context.getStore();
  if (current?.runId === undefined || current.runEventId !== undefined) return await fn();

  let event: RunEventSummary;
  try {
    event = await client.createRunEvent(current.runId, req);
  } catch (error) {
    if (error instanceof SdkError) {
      notifyAutomaticRunWarning(opts, 'event', error);
      return await fn();
    }
    throw error;
  }

  return await context.run({ ...current, runEventId: event.id }, fn);
}

async function startAutomaticRun(client: Client, opts: AutomaticRunOptions): Promise<RunSummary> {
  const externalId = await resolveAutomaticRunExternalId(opts);
  return client.startRun({
    agent_id: opts.agentId,
    kind: opts.kind,
    metadata: opts.metadata,
    ...(externalId === undefined ? {} : { external_id: externalId }),
  });
}

async function resolveAutomaticRunExternalId(
  opts: AutomaticRunOptions,
): Promise<string | undefined> {
  if (opts.externalId === undefined) return undefined;
  const raw = typeof opts.externalId === 'function' ? await opts.externalId() : opts.externalId;
  const externalId = raw.trim();
  if (opts.scope === 'session' && externalId.length === 0) {
    throw new TypeError('guardAgent session run externalId must be a non-empty string');
  }
  return externalId.length === 0 ? undefined : externalId;
}

async function withRunId<T>(runId: string, fn: () => Promise<T>): Promise<T> {
  const context = await runContext();
  const nextContext = { ...context.getStore(), runId };
  delete nextContext.runEventId;
  return context.run(nextContext, fn);
}

function notifyAutomaticRunWarning(
  opts: AutomaticRunOptions,
  phase: AutomaticRunWarning['phase'],
  error: SdkError,
): void {
  try {
    opts.onLifecycleWarning?.({
      code:
        phase === 'start'
          ? 'run_start_failed'
          : phase === 'event'
            ? 'run_event_create_failed'
            : 'run_finish_failed',
      phase,
      error,
    });
  } catch {
    // An observability warning hook must not replace the guarded result.
  }
}

export interface GuardToolCallOptions {
  agentId: string;
  operation: string;
  parameters?: Record<string, unknown>;
  sideEffect?: SideEffectClass;
  sources?: Source[];
  provenance?: ProvenanceMap;
  context?: Record<string, unknown> | null;
  eventKind?: EventKind;
  invocationId?: string;
  toolIdentity?: ToolIdentity;
}

export interface AuthorizedActionOptions extends GuardToolCallOptions {
  toolIdentity: ToolIdentity;
  timeoutMs?: number;
  pollIntervalMs?: number;
  signal?: AbortSignal;
}

export interface AuthorizedShellActionOptions extends Omit<
  AuthorizedActionOptions,
  'operation' | 'parameters' | 'sideEffect' | 'eventKind'
> {
  command: string;
  shell?: ShellLanguage;
  cwd?: string;
  workspaceRoot?: string;
  commandTimeoutMs?: number;
  runInBackground?: boolean;
}

export interface AuthorizedActionResult<T> {
  decision: AuthorizationDecision;
  executed: boolean;
  value?: T;
}

export interface FinancialOperationRunOptions {
  execute?: boolean;
  signal?: AbortSignal;
}

export interface FinancialOperationSpec<Input, Facts = undefined> {
  operation: string;
  kind: FinancialActionKind;
  principalId: string;
  rail: FinancialRail;
  amount: (input: Input, facts: Facts) => MoneyAmount;
  idempotencyKey: (input: Input, facts: Facts) => string;
  counterparty?: (input: Input, facts: Facts) => CounterpartyRef | undefined;
  authorization?: (input: Input, facts: Facts) => AuthorizationClaim | undefined;
  memo?: (input: Input, facts: Facts) => string | undefined;
  metadata?: (input: Input, facts: Facts) => Record<string, unknown> | null | undefined;
  evidence?: (input: Input, facts: Facts) => EvidenceRef[] | undefined;
  execute?: boolean;
}

export interface FinancialOperation<Input, Facts = undefined> {
  buildRequest(
    input: Input,
    facts?: Facts,
    options?: FinancialOperationRunOptions,
  ): CreateFinancialActionRequest;
  verify(
    input: Input,
    facts?: Facts,
    options?: FinancialOperationRunOptions,
  ): Promise<FinancialActionRecord>;
}

export interface ListTracesOptions {
  limit?: number;
  sessionId?: string;
}

export class Client {
  private readonly baseUrl: string;
  private readonly apiKey: string | undefined;
  private readonly fetchImpl: typeof fetch;
  private readonly retry: RetryConfig;
  private readonly onRetry: ClientOptions['onRetry'];

  constructor(opts: ClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/$/, '');
    this.apiKey = opts.apiKey;
    this.fetchImpl = opts.fetchImpl ?? globalThis.fetch.bind(globalThis);
    this.retry = opts.retry ?? DEFAULT_RETRY;
    this.onRetry = opts.onRetry;
  }

  /**
   * Submit a full `GuardEvent` (sources + provenance) for a runtime
   * decision. The `checks` and `signals` fields are server-populated;
   * client-supplied values are ignored.
   */
  async submitEvent(event: GuardEvent, signal?: AbortSignal): Promise<AuthorizationDecision> {
    const body = await this.withActiveContext(event);
    return this.withRetry(
      (signal) =>
        this.sendJson<AuthorizationDecision>(
          '/v1/events',
          {
            method: 'POST',
            body: stringifyJson(body),
          },
          signal,
        ),
      signal,
    );
  }

  async getApproval(approvalId: string, signal?: AbortSignal): Promise<AuthorizationApproval> {
    return this.withRetry(
      (retrySignal) =>
        this.sendJson<AuthorizationApproval>(
          `/v1/authorization/approvals/${encodeURIComponent(approvalId)}`,
          { method: 'GET' },
          retrySignal,
        ),
      signal,
    );
  }

  async resumeAuthorizedAction(
    event: GuardEvent,
    grantId: string,
    attemptId: string,
    signal?: AbortSignal,
  ): Promise<AuthorizationDecision> {
    const resumed = cloneEvent(event);
    resumed.action.authorization = {
      grant_id: grantId,
      attempt_id: attemptId,
    };
    return this.submitEvent(resumed, signal);
  }

  async withAuthorizedAction<T>(
    opts: AuthorizedActionOptions,
    execute: (parameters: Readonly<Record<string, unknown>>) => Promise<T>,
  ): Promise<AuthorizedActionResult<T>> {
    const event = cloneEvent({
      kind: opts.eventKind ?? 'tool.call.proposed',
      principal: {
        workspace_id: '',
        environment_id: '',
        agent_id: opts.agentId,
      },
      action: {
        operation: opts.operation,
        parameters: opts.parameters ?? {},
        ...(opts.sideEffect ? { side_effect: opts.sideEffect } : {}),
        invocation_id: opts.invocationId ?? newUuid(),
        tool_identity: opts.toolIdentity,
      },
      sources: opts.sources ?? [],
      provenance: opts.provenance ?? {},
      context: opts.context ?? null,
    });
    const approvedParameters = event.action.parameters ?? {};
    deepFreeze(approvedParameters);
    const executePermitted = async (
      permitted: AuthorizationDecision,
    ): Promise<AuthorizedActionResult<T>> => {
      try {
        const value = await execute(approvedParameters);
        if (permitted.lease) {
          await this.completeLease(permitted.lease.id, {
            status: 'consumed',
            outcome: { success: true },
          });
        }
        return { decision: permitted, executed: true, value };
      } catch (error) {
        if (permitted.lease) {
          try {
            await this.completeLease(permitted.lease.id, {
              status: 'canceled',
              outcome: { success: false },
            });
          } catch {
            // Preserve the callback/completion error. A caller can reconcile
            // the claimed lease without ever running the callback again.
          }
        }
        throw error;
      }
    };
    let decision = await this.submitEvent(event, opts.signal);
    if (decision.effect === 'permit') {
      return executePermitted(decision);
    }
    if (decision.effect !== 'require_approval' || !decision.approval) {
      return { decision, executed: false };
    }

    const approvalId = decision.approval.id;
    const attemptId = newUuid();
    const deadline = Date.now() + (opts.timeoutMs ?? 60_000);
    while (Date.now() < deadline) {
      if (opts.signal?.aborted) throw opts.signal.reason;
      const approval = await this.getApproval(approvalId, opts.signal);
      if (approval.status === 'approved' && approval.grant_id) {
        decision = await this.resumeAuthorizedAction(
          event,
          approval.grant_id,
          attemptId,
          opts.signal,
        );
        if (decision.effect === 'permit' && decision.lease) {
          return executePermitted(decision);
        }
        return { decision, executed: false };
      }
      if (approval.status !== 'pending') return { decision, executed: false };
      await abortableDelay(opts.pollIntervalMs ?? 1_000, opts.signal);
    }
    return { decision, executed: false };
  }

  async withAuthorizedShellAction<T>(
    opts: AuthorizedShellActionOptions,
    execute: (parameters: Readonly<ShellActionParameters>) => Promise<T>,
  ): Promise<AuthorizedActionResult<T>> {
    const parameters: ShellActionParameters = {
      command: opts.command,
      shell: opts.shell ?? 'bash',
      run_in_background: opts.runInBackground ?? false,
      ...(opts.cwd ? { cwd: opts.cwd } : {}),
      ...(opts.workspaceRoot ? { workspace_root: opts.workspaceRoot } : {}),
      ...(opts.commandTimeoutMs !== undefined ? { timeout_ms: BigInt(opts.commandTimeoutMs) } : {}),
    };
    return this.withAuthorizedAction(
      {
        ...opts,
        operation: 'Bash',
        parameters,
        sideEffect: 'shell_exec',
        eventKind: 'shell.action.proposed',
      },
      async () => execute(parameters),
    );
  }

  async withRun<T>(opts: WithRunOptions, fn: (run: ActiveRun) => Promise<T>): Promise<T> {
    const metadata = opts.inputSummary
      ? { ...(opts.metadata ?? {}), input_summary: opts.inputSummary }
      : (opts.metadata ?? {});
    const req: Omit<CreateRunRequest, 'metadata'> & { metadata?: Record<string, unknown> } = {
      agent_id: opts.agentId,
      kind: opts.kind ?? 'other',
      metadata,
    };
    if (opts.externalId) req.external_id = opts.externalId;
    const summary = await this.startRun(req);
    let finished = false;
    const run: ActiveRun = {
      id: summary.id,
      withEvent: async (req, eventFn) => {
        const event = await this.createRunEvent(summary.id, req);
        const context = await runContext();
        return context.run(
          { ...context.getStore(), runId: summary.id, runEventId: event.id },
          eventFn,
        );
      },
      finish: async (status = 'completed') => {
        await this.finishRun(summary.id, status);
        finished = true;
      },
    };

    const context = await runContext();
    const nextContext = { ...context.getStore(), runId: summary.id };
    delete nextContext.runEventId;
    return context.run(nextContext, async () => {
      try {
        const result = await fn(run);
        if (!finished) await run.finish('completed');
        return result;
      } catch (error) {
        if (!finished && opts.finishOnError !== false) {
          try {
            await run.finish('failed');
          } catch {
            // Keep the application/guard failure as the error callers receive.
          }
        }
        throw error;
      }
    });
  }

  async guardToolCall(
    opts: GuardToolCallOptions,
    signal?: AbortSignal,
  ): Promise<AuthorizationDecision> {
    return this.submitEvent(
      {
        kind: opts.eventKind ?? 'tool.call.proposed',
        principal: {
          workspace_id: '',
          environment_id: '',
          agent_id: opts.agentId,
        },
        action: {
          operation: opts.operation,
          parameters: opts.parameters ?? {},
          ...(opts.sideEffect ? { side_effect: opts.sideEffect } : {}),
          invocation_id: opts.invocationId ?? newUuid(),
          tool_identity:
            opts.toolIdentity ??
            ({
              server_id: 'trustloopguard-sdk',
              tool_name: opts.operation,
              schema_hash: 'sdk-legacy-untyped-v1',
            } satisfies ToolIdentity),
        },
        sources: opts.sources ?? [],
        provenance: opts.provenance ?? {},
        context: opts.context ?? null,
      },
      signal,
    );
  }

  async guardShellCommand(
    opts: Omit<AuthorizedShellActionOptions, 'timeoutMs' | 'pollIntervalMs' | 'signal'>,
    signal?: AbortSignal,
  ): Promise<AuthorizationDecision> {
    return this.guardToolCall(
      {
        agentId: opts.agentId,
        operation: 'Bash',
        parameters: {
          command: opts.command,
          shell: opts.shell ?? 'bash',
          run_in_background: opts.runInBackground ?? false,
          ...(opts.cwd ? { cwd: opts.cwd } : {}),
          ...(opts.workspaceRoot ? { workspace_root: opts.workspaceRoot } : {}),
          ...(opts.commandTimeoutMs !== undefined
            ? { timeout_ms: BigInt(opts.commandTimeoutMs) }
            : {}),
        },
        sideEffect: 'shell_exec',
        eventKind: 'shell.action.proposed',
        toolIdentity: opts.toolIdentity,
        ...(opts.invocationId ? { invocationId: opts.invocationId } : {}),
        ...(opts.sources ? { sources: opts.sources } : {}),
        ...(opts.provenance ? { provenance: opts.provenance } : {}),
        ...(opts.context !== undefined ? { context: opts.context } : {}),
      },
      signal,
    );
  }

  async listApprovals(signal?: AbortSignal): Promise<AuthorizationApprovalListResponse> {
    return this.withRetry(
      (retrySignal) =>
        this.sendJson<AuthorizationApprovalListResponse>(
          '/v1/authorization/approvals',
          { method: 'GET' },
          retrySignal,
        ),
      signal,
    );
  }

  async decideApproval(
    approvalId: string,
    request: DecideAuthorizationApprovalRequest,
    signal?: AbortSignal,
  ): Promise<DecideAuthorizationApprovalResponse> {
    return this.sendJson<DecideAuthorizationApprovalResponse>(
      `/v1/authorization/approvals/${encodeURIComponent(approvalId)}/decide`,
      { method: 'POST', body: stringifyJson(request) },
      signal,
    );
  }

  async createGrant(
    request: CreateAuthorizationGrantRequest,
    signal?: AbortSignal,
  ): Promise<AuthorizationGrant> {
    return this.sendJson<AuthorizationGrant>(
      '/v1/authorization/grants',
      { method: 'POST', body: stringifyJson(request) },
      signal,
    );
  }

  async listGrants(signal?: AbortSignal): Promise<AuthorizationGrantListResponse> {
    return this.withRetry(
      (retrySignal) =>
        this.sendJson<AuthorizationGrantListResponse>(
          '/v1/authorization/grants',
          { method: 'GET' },
          retrySignal,
        ),
      signal,
    );
  }

  async revokeGrant(grantId: string, signal?: AbortSignal): Promise<AuthorizationGrant> {
    return this.sendJson<AuthorizationGrant>(
      `/v1/authorization/grants/${encodeURIComponent(grantId)}/revoke`,
      { method: 'POST', body: '{}' },
      signal,
    );
  }

  async completeLease(
    leaseId: string,
    request: CompleteAuthorizationLeaseRequest,
    signal?: AbortSignal,
  ): Promise<void> {
    await this.sendJson(
      `/v1/authorization/leases/${encodeURIComponent(leaseId)}/complete`,
      { method: 'POST', body: stringifyJson(request) },
      signal,
    );
  }

  async getAuthorizationReceipt(
    receiptId: string,
    signal?: AbortSignal,
  ): Promise<AuthorizationReceipt> {
    return this.withRetry(
      (retrySignal) =>
        this.sendJson<AuthorizationReceipt>(
          `/v1/authorization/receipts/${encodeURIComponent(receiptId)}`,
          { method: 'GET' },
          retrySignal,
        ),
      signal,
    );
  }

  async listAuthorizationReceipts(signal?: AbortSignal): Promise<AuthorizationReceiptListResponse> {
    return this.withRetry(
      (retrySignal) =>
        this.sendJson<AuthorizationReceiptListResponse>(
          '/v1/authorization/receipts',
          { method: 'GET' },
          retrySignal,
        ),
      signal,
    );
  }

  async verifyAction(
    req: CreateFinancialActionRequest,
    signal?: AbortSignal,
  ): Promise<FinancialActionRecord> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialActionRecord>(
          '/v1/financial/actions',
          {
            method: 'POST',
            body: stringifyJson(req),
          },
          signal,
        ),
      signal,
    );
  }

  async guardPayment(
    req: CreateFinancialActionRequest,
    signal?: AbortSignal,
  ): Promise<FinancialActionRecord> {
    return this.verifyAction(req, signal);
  }

  financialOperation<Input, Facts = undefined>(
    spec: FinancialOperationSpec<Input, Facts>,
  ): FinancialOperation<Input, Facts> {
    const operation = cleanFinancialOperationField('operation', spec.operation);
    const principalId = cleanFinancialOperationField('principalId', spec.principalId);
    return {
      buildRequest: (input, facts, options) =>
        buildFinancialOperationRequest(
          input,
          facts as Facts,
          spec,
          operation,
          principalId,
          options,
        ),
      verify: (input, facts, options) =>
        this.verifyAction(
          buildFinancialOperationRequest(
            input,
            facts as Facts,
            spec,
            operation,
            principalId,
            options,
          ),
          options?.signal,
        ),
    };
  }

  async getFinancialAction(actionId: string, signal?: AbortSignal): Promise<FinancialActionRecord> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialActionRecord>(
          `/v1/financial/actions/${encodeURIComponent(actionId)}`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async listFinancialActions(signal?: AbortSignal): Promise<FinancialActionListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialActionListResponse>(
          '/v1/financial/actions',
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async authorizeAgenticPayment(
    req: AgenticPaymentAuthorizeRequest,
    signal?: AbortSignal,
  ): Promise<AgenticPaymentAuthorizationResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<AgenticPaymentAuthorizationResponse>(
          '/v1/financial/agentic-payments/authorize',
          {
            method: 'POST',
            body: stringifyJson(req),
          },
          signal,
        ),
      signal,
    );
  }

  async getAgenticPayment(actionId: string, signal?: AbortSignal): Promise<AgenticPaymentRecord> {
    return this.withRetry(
      (signal) =>
        this.sendJson<AgenticPaymentRecord>(
          `/v1/financial/agentic-payments/${encodeURIComponent(actionId)}`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async commitAgenticPayment(
    actionId: string,
    req: AgenticPaymentCommitRequest,
    signal?: AbortSignal,
  ): Promise<AgenticPaymentRecord> {
    return this.sendJson<AgenticPaymentRecord>(
      `/v1/financial/agentic-payments/${encodeURIComponent(actionId)}/commit`,
      {
        method: 'POST',
        body: stringifyJson(req),
      },
      signal,
    );
  }

  async rollbackAgenticPayment(
    actionId: string,
    req: AgenticPaymentRollbackRequest,
    signal?: AbortSignal,
  ): Promise<AgenticPaymentRecord> {
    return this.sendJson<AgenticPaymentRecord>(
      `/v1/financial/agentic-payments/${encodeURIComponent(actionId)}/rollback`,
      {
        method: 'POST',
        body: stringifyJson(req),
      },
      signal,
    );
  }

  async getAgenticPaymentReceipt(
    actionId: string,
    signal?: AbortSignal,
  ): Promise<FinancialReceipt> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialReceipt>(
          `/v1/financial/agentic-payments/${encodeURIComponent(actionId)}/receipt`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async createFinancialPolicy(
    req: CreateFinancialPolicyRequest,
    signal?: AbortSignal,
  ): Promise<FinancialPolicyRecord> {
    return this.sendJson<FinancialPolicyRecord>(
      '/v1/financial/policies',
      {
        method: 'POST',
        body: stringifyJson(req),
      },
      signal,
    );
  }

  async listFinancialPolicies(signal?: AbortSignal): Promise<FinancialPolicyListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialPolicyListResponse>(
          '/v1/financial/policies',
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async getReceipt(receiptId: string, signal?: AbortSignal): Promise<FinancialReceipt> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialReceipt>(
          `/v1/financial/receipts/${encodeURIComponent(receiptId)}`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async recordActionOutcome(
    actionId: string,
    outcome: FinancialActionOutcome,
    signal?: AbortSignal,
  ): Promise<FinancialActionOutcome> {
    return this.sendJson<FinancialActionOutcome>(
      `/v1/financial/actions/${encodeURIComponent(actionId)}/outcomes`,
      {
        method: 'POST',
        body: stringifyJson(outcome),
      },
      signal,
    );
  }

  async listActionOutcomes(
    actionId: string,
    signal?: AbortSignal,
  ): Promise<FinancialOutcomeListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialOutcomeListResponse>(
          `/v1/financial/actions/${encodeURIComponent(actionId)}/outcomes`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async executeAction(
    actionId: string,
    request: ExecuteFinancialActionRequest = {},
    signal?: AbortSignal,
  ): Promise<FinancialActionRecord> {
    return this.sendJson<FinancialActionRecord>(
      `/v1/financial/actions/${encodeURIComponent(actionId)}/execute`,
      { method: 'POST', body: stringifyJson(request) },
      signal,
    );
  }

  async startRun(
    req: Omit<CreateRunRequest, 'metadata'> & { metadata?: Record<string, unknown> },
    signal?: AbortSignal,
  ): Promise<RunSummary> {
    return this.withRetry(
      (signal) =>
        this.sendJson<RunSummary>(
          '/v1/runs',
          {
            method: 'POST',
            body: JSON.stringify({ metadata: {}, ...req }),
          },
          signal,
        ),
      signal,
    );
  }

  private async withActiveContext(event: GuardEvent): Promise<GuardEvent> {
    const context = (await runContext()).getStore();
    if (!context?.runId && !context?.runEventId) return event;
    const principal = { ...event.principal };
    if (!principal.run_id && context.runId) principal.run_id = context.runId;
    if (!principal.run_event_id && principal.run_id === context.runId && context.runEventId) {
      principal.run_event_id = context.runEventId;
    }
    return { ...event, principal };
  }

  async listRuns(signal?: AbortSignal): Promise<RunListResponse> {
    return this.withRetry(
      (signal) => this.sendJson<RunListResponse>('/v1/runs', { method: 'GET' }, signal),
      signal,
    );
  }

  async getRun(runId: string, signal?: AbortSignal): Promise<RunDetail> {
    return this.withRetry(
      (signal) =>
        this.sendJson<RunDetail>(
          `/v1/runs/${encodeURIComponent(runId)}`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async updateRun(runId: string, req: UpdateRunRequest, signal?: AbortSignal): Promise<RunSummary> {
    return this.withRetry(
      (signal) =>
        this.sendJson<RunSummary>(
          `/v1/runs/${encodeURIComponent(runId)}`,
          {
            method: 'PATCH',
            body: JSON.stringify(req),
          },
          signal,
        ),
      signal,
    );
  }

  async finishRun(
    runId: string,
    status: Extract<RunStatus, 'completed' | 'failed' | 'canceled'> = 'completed',
    signal?: AbortSignal,
  ): Promise<RunSummary> {
    return this.updateRun(runId, { status }, signal);
  }

  async createRunEvent(
    runId: string,
    req: Omit<CreateRunEventRequest, 'metadata'> & { metadata?: Record<string, unknown> },
    signal?: AbortSignal,
  ): Promise<RunEventSummary> {
    return this.withRetry(
      (signal) =>
        this.sendJson<RunEventSummary>(
          `/v1/runs/${encodeURIComponent(runId)}/events`,
          {
            method: 'POST',
            body: JSON.stringify({ metadata: {}, ...req }),
          },
          signal,
        ),
      signal,
    );
  }

  async listRunEvents(runId: string, signal?: AbortSignal): Promise<RunEventListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<RunEventListResponse>(
          `/v1/runs/${encodeURIComponent(runId)}/events`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async listRunTraces(runId: string, signal?: AbortSignal): Promise<TraceListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<TraceListResponse>(
          `/v1/runs/${encodeURIComponent(runId)}/traces`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async listTraces(
    options: ListTracesOptions = {},
    signal?: AbortSignal,
  ): Promise<TraceListResponse> {
    const query = new URLSearchParams();
    if (options.limit !== undefined) query.set('limit', String(options.limit));
    if (options.sessionId !== undefined) query.set('session_id', options.sessionId);
    const suffix = query.size > 0 ? `?${query.toString()}` : '';
    return this.withRetry(
      (signal) =>
        this.sendJson<TraceListResponse>(`/v1/traces${suffix}`, { method: 'GET' }, signal),
      signal,
    );
  }

  async validatePolicy(source: string, signal?: AbortSignal): Promise<PolicyValidateResponse> {
    return this.withRetry(
      (signal) =>
        this.sendText<PolicyValidateResponse>(
          '/v1/policies/validate',
          'POST',
          source,
          'application/yaml',
          signal,
        ),
      signal,
    );
  }

  async listPolicies(
    optionsOrSignal: { family?: PolicyFamily } | AbortSignal = {},
    maybeSignal?: AbortSignal,
  ): Promise<PolicyListResponse> {
    const options =
      typeof AbortSignal !== 'undefined' && optionsOrSignal instanceof AbortSignal
        ? {}
        : (optionsOrSignal as { family?: PolicyFamily });
    const signal =
      typeof AbortSignal !== 'undefined' && optionsOrSignal instanceof AbortSignal
        ? optionsOrSignal
        : maybeSignal;
    const query = new URLSearchParams();
    if (options.family !== undefined) query.set('family', options.family);
    const suffix = query.size > 0 ? `?${query.toString()}` : '';
    return this.withRetry(
      (signal) =>
        this.sendJson<PolicyListResponse>(`/v1/policies${suffix}`, { method: 'GET' }, signal),
      signal,
    );
  }

  async listAgents(signal?: AbortSignal): Promise<AgentListResponse> {
    return this.withRetry(
      (signal) => this.sendJson<AgentListResponse>('/v1/agents', { method: 'GET' }, signal),
      signal,
    );
  }

  async upsertAgent(profile: AgentProfile, signal?: AbortSignal): Promise<AgentProfile> {
    return this.withRetry(
      (signal) =>
        this.sendJson<AgentProfile>(
          '/v1/agents',
          {
            method: 'POST',
            body: JSON.stringify(profile),
          },
          signal,
        ),
      signal,
    );
  }

  async listToolMetadata(signal?: AbortSignal): Promise<ToolMetadataListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<ToolMetadataListResponse>('/v1/tool-metadata', { method: 'GET' }, signal),
      signal,
    );
  }

  async upsertToolMetadata(
    req: UpsertToolMetadataRequest,
    signal?: AbortSignal,
  ): Promise<ToolMetadataEntry> {
    return this.withRetry(
      (signal) =>
        this.sendJson<ToolMetadataEntry>(
          '/v1/tool-metadata',
          {
            method: 'POST',
            body: JSON.stringify(req),
          },
          signal,
        ),
      signal,
    );
  }

  async getPolicy(policyId: string, signal?: AbortSignal): Promise<PolicyDocument> {
    return this.withRetry(
      (signal) =>
        this.sendJson<PolicyDocument>(
          `/v1/policies/${encodeURIComponent(policyId)}`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async upsertPolicy(source: string, signal?: AbortSignal): Promise<PolicyDocument> {
    return this.withRetry(
      (signal) =>
        this.sendText<PolicyDocument>('/v1/policies', 'POST', source, 'application/yaml', signal),
      signal,
    );
  }

  async setPolicyEnabled(
    policyId: string,
    enabled: boolean,
    signal?: AbortSignal,
  ): Promise<PolicyDocument> {
    return this.withRetry(
      (signal) =>
        this.sendJson<PolicyDocument>(
          `/v1/policies/${encodeURIComponent(policyId)}/enabled`,
          {
            method: 'PATCH',
            body: JSON.stringify({ enabled }),
          },
          signal,
        ),
      signal,
    );
  }

  async batchSetPolicyEnabled(
    policyIds: string[],
    enabled: boolean,
    signal?: AbortSignal,
  ): Promise<PolicyBatchSetEnabledResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<PolicyBatchSetEnabledResponse>(
          '/v1/policies/batch/enabled',
          {
            method: 'PATCH',
            body: JSON.stringify({ ids: policyIds, enabled }),
          },
          signal,
        ),
      signal,
    );
  }

  /**
   * LLM-draft a policy skeleton from a natural-language prompt. The
   * server holds the provider key; the response is a strict, typed
   * `PolicyDraftResponse`. Returns a 503-mapped `Unavailable` error when
   * the deployment has no LLM configured.
   */
  async draftPolicy(prompt: string, signal?: AbortSignal): Promise<PolicyDraftResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<PolicyDraftResponse>(
          '/v1/policies/draft',
          {
            method: 'POST',
            body: JSON.stringify({ prompt }),
          },
          signal,
        ),
      signal,
    );
  }

  /**
   * Derive a guardrail policy set from the agent's stored `system_prompt`,
   * auto-persist each draft with `enabled=false`, and return what was
   * saved. The caller must have previously registered the agent (with a
   * non-empty `system_prompt`) via `POST /v1/agents`.
   *
   * Errors:
   * - `NotFound` (404) — agent is not registered.
   * - `Unprocessable` (422) — agent has no `system_prompt`.
   * - `Unavailable` (503) — the deployment has no LLM configured.
   */
  async generateGuardrails(
    agentId: string,
    signal?: AbortSignal,
  ): Promise<GuardrailGenerateResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<GuardrailGenerateResponse>(
          `/v1/agents/${encodeURIComponent(agentId)}/guardrails/generate`,
          { method: 'POST' },
          signal,
        ),
      signal,
    );
  }

  /**
   * Derive **tailored** attack vectors from the agent's own definition — its
   * chat `system_prompt` and/or imported `workflow_definition` — and **save**
   * the result as a named plan. For workflow agents the response also carries
   * the static analyzer's injectable `source → sink` paths. Feed the vectors
   * into a red-team dispatch as seeds; re-select the saved plan later via
   * {@link listPlans}.
   *
   * Errors:
   * - `NotFound` (404) — agent is not registered.
   * - `Unprocessable` (422) — agent has neither a `system_prompt` nor a workflow.
   * - `Unavailable` (503) — the deployment has no LLM configured.
   */
  async planAttackVectors(
    agentId: string,
    request: RedteamPlanRequest = {},
    signal?: AbortSignal,
  ): Promise<RedteamPlanResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<RedteamPlanResponse>(
          `/v1/agents/${encodeURIComponent(agentId)}/redteam/plan`,
          { method: 'POST', body: JSON.stringify(request) },
          signal,
        ),
      signal,
    );
  }

  /** List an agent's saved attack plans, newest first. */
  async listPlans(agentId: string, signal?: AbortSignal): Promise<RedteamPlanListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<RedteamPlanListResponse>(
          `/v1/agents/${encodeURIComponent(agentId)}/redteam/plans`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  /** Delete a saved attack plan by id. */
  async deletePlan(planId: string, signal?: AbortSignal): Promise<void> {
    return this.withRetry(
      (signal) =>
        this.sendJson<void>(
          `/v1/redteam/plans/${encodeURIComponent(planId)}`,
          { method: 'DELETE' },
          signal,
        ),
      signal,
    );
  }

  /**
   * Synthesize **preventive** guardrails from the agent's imported workflow —
   * one per unguarded `source → sink` path — and attach them `enabled=false`.
   * The static (no-execution) twin of harden, for agents without a runnable
   * target. No injectable path ⇒ an empty set.
   *
   * Errors:
   * - `NotFound` (404) — agent is not registered.
   * - `Unprocessable` (422) — agent has no `workflow_definition`.
   */
  async generateStaticPolicies(
    agentId: string,
    signal?: AbortSignal,
  ): Promise<GuardrailGenerateResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<GuardrailGenerateResponse>(
          `/v1/agents/${encodeURIComponent(agentId)}/redteam/static-policies`,
          { method: 'POST' },
          signal,
        ),
      signal,
    );
  }

  /**
   * List policies owned by an agent. Empty when the agent has none or
   * doesn't exist — existence is the caller's concern.
   */
  async listGuardrails(agentId: string, signal?: AbortSignal): Promise<GuardrailListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<GuardrailListResponse>(
          `/v1/agents/${encodeURIComponent(agentId)}/guardrails`,
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async deletePolicy(policyId: string, signal?: AbortSignal): Promise<void> {
    return this.withRetry(
      (signal) =>
        this.sendJson<void>(
          `/v1/policies/${encodeURIComponent(policyId)}`,
          { method: 'DELETE' },
          signal,
        ),
      signal,
    );
  }

  async batchRevokeApiKeys(
    apiKeyIds: string[],
    signal?: AbortSignal,
  ): Promise<ApiKeyBatchRevokeResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<ApiKeyBatchRevokeResponse>(
          '/v1/api-keys/batch/revoke',
          {
            method: 'PATCH',
            body: JSON.stringify({ ids: apiKeyIds }),
          },
          signal,
        ),
      signal,
    );
  }

  private async withRetry<T>(
    send: (signal?: AbortSignal) => Promise<T>,
    signal?: AbortSignal,
  ): Promise<T> {
    const start = performance.now();
    let attempt = 0;
    while (true) {
      attempt += 1;
      try {
        return await send(signal);
      } catch (e) {
        if (!(e instanceof SdkError)) throw e;
        const elapsedS = (performance.now() - start) / 1000;
        const delay = nextDelay(this.retry, attempt, elapsedS, e, Math.random());
        if (delay === undefined) throw e;
        this.onRetry?.({ attempt, delayS: delay, error: e });
        await new Promise((resolve) => setTimeout(resolve, delay * 1000));
      }
    }
  }

  private async sendText<T>(
    path: string,
    method: string,
    body: string,
    contentType: string,
    signal?: AbortSignal,
  ): Promise<T> {
    return this.sendJson<T>(
      path,
      {
        method,
        headers: { 'content-type': contentType },
        body,
      },
      signal,
    );
  }

  private async sendJson<T>(path: string, init: RequestInit, signal?: AbortSignal): Promise<T> {
    const headers: Record<string, string> = {
      'content-type': 'application/json',
      ...((init.headers as Record<string, string> | undefined) ?? {}),
    };
    if (this.apiKey !== undefined) {
      headers['authorization'] = `Bearer ${this.apiKey}`;
    }

    const requestInit: RequestInit = {
      ...init,
      headers,
    };
    if (signal !== undefined) {
      requestInit.signal = signal;
    }

    let res: Response;
    try {
      res = await this.fetchImpl(`${this.baseUrl}${path}`, requestInit);
    } catch (e) {
      throw new Transport(e instanceof Error ? e.message : String(e));
    }

    if (res.status === 204) return undefined as T;

    if (res.ok) {
      try {
        return (await res.json()) as T;
      } catch (e) {
        throw new Decode(`failed to parse response: ${String(e)}`);
      }
    }

    const retryAfter = parseRetryAfter(res.headers.get('retry-after'));
    const body = await res.text().catch(() => '');
    throw fromResponse(res.status, body, retryAfter);
  }
}

function cloneEvent(event: GuardEvent): GuardEvent {
  return JSON.parse(JSON.stringify(event)) as GuardEvent;
}

function deepFreeze(value: object): void {
  Object.freeze(value);
  for (const nested of Object.values(value)) {
    if (nested !== null && typeof nested === 'object' && !Object.isFrozen(nested)) {
      deepFreeze(nested);
    }
  }
}

function newUuid(): string {
  return globalThis.crypto.randomUUID();
}

function abortableDelay(delayMs: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(signal.reason);
      return;
    }
    const timer = setTimeout(resolve, delayMs);
    signal?.addEventListener(
      'abort',
      () => {
        clearTimeout(timer);
        reject(signal.reason);
      },
      { once: true },
    );
  });
}

function stringifyJson(value: Parameters<typeof JSON.stringify>[0]): string {
  return JSON.stringify(value, (_key, nested) => {
    if (typeof nested !== 'bigint') return nested;
    const asNumber = Number(nested);
    if (!Number.isSafeInteger(asNumber)) {
      throw new TypeError('Cannot serialize bigint outside the safe JSON integer range');
    }
    return asNumber;
  });
}

function buildFinancialOperationRequest<Input, Facts>(
  input: Input,
  facts: Facts,
  spec: FinancialOperationSpec<Input, Facts>,
  operation: string,
  principalId: string,
  options?: FinancialOperationRunOptions,
): CreateFinancialActionRequest {
  const metadata = spec.metadata?.(input, facts) ?? {};
  const action: CreateFinancialActionRequest['action'] = {
    kind: spec.kind,
    operation,
    principal_id: principalId,
    amount: spec.amount(input, facts),
    rail: spec.rail,
    metadata,
  };
  const counterparty = spec.counterparty?.(input, facts);
  if (counterparty !== undefined) action.counterparty = counterparty;
  const memo = spec.memo?.(input, facts);
  if (memo !== undefined) action.memo = memo;

  const request: CreateFinancialActionRequest = {
    idempotency_key: cleanFinancialOperationField(
      'idempotencyKey',
      spec.idempotencyKey(input, facts),
    ),
    execute: options?.execute ?? spec.execute ?? false,
    action,
    evidence: spec.evidence?.(input, facts) ?? [],
  };
  const authorization = spec.authorization?.(input, facts);
  if (authorization !== undefined) request.authorization = authorization;
  return request;
}

function cleanFinancialOperationField(name: string, value: string): string {
  const trimmed = value.trim();
  if (trimmed === '') throw new TypeError(`${name} must not be empty`);
  return trimmed;
}
