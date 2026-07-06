// Thin HTTP client over the public GuardEvent runtime contract.
//
// Retry policy mirrors tl-sdk-rust and the Python SDK — same defaults,
// same `nextDelay` semantics. Voice callers should pass
// `{ ...DEFAULT_RETRY, maxAttempts: 1 }` to opt out.

import type { Decision } from './generated/Decision';
import type { GuardEvent } from './generated/GuardEvent';
import type { AgentListResponse } from './generated/AgentListResponse';
import type { AgentProfile } from './generated/AgentProfile';
import type { ApiKeyBatchRevokeResponse } from './generated/ApiKeyBatchRevokeResponse';
import type { GuardrailGenerateResponse } from './generated/GuardrailGenerateResponse';
import type { GuardrailListResponse } from './generated/GuardrailListResponse';
import type { RedteamPlanRequest } from './generated/RedteamPlanRequest';
import type { RedteamPlanResponse } from './generated/RedteamPlanResponse';
import type { RedteamPlanListResponse } from './generated/RedteamPlanListResponse';
import type { PolicyDocument } from './generated/PolicyDocument';
import type { PolicyBatchSetEnabledResponse } from './generated/PolicyBatchSetEnabledResponse';
import type { PolicyDraftResponse } from './generated/PolicyDraftResponse';
import type { PolicyListResponse } from './generated/PolicyListResponse';
import type { PolicyValidateResponse } from './generated/PolicyValidateResponse';
import type { CreateFinancialActionRequest } from './generated/CreateFinancialActionRequest';
import type { CreateFinancialMandateRequest } from './generated/CreateFinancialMandateRequest';
import type { FinancialActionListResponse } from './generated/FinancialActionListResponse';
import type { FinancialMandate } from './generated/FinancialMandate';
import type { FinancialMandateListResponse } from './generated/FinancialMandateListResponse';
import type { FinancialReceipt } from './generated/FinancialReceipt';
import type { CreateRunEventRequest } from './generated/CreateRunEventRequest';
import type { CreateRunRequest } from './generated/CreateRunRequest';
import type { RunDetail } from './generated/RunDetail';
import type { RunEventListResponse } from './generated/RunEventListResponse';
import type { RunEventSummary } from './generated/RunEventSummary';
import type { RunKind } from './generated/RunKind';
import type { RunListResponse } from './generated/RunListResponse';
import type { RunStatus } from './generated/RunStatus';
import type { RunSummary } from './generated/RunSummary';
import type { FinancialActionRecord } from './generated/FinancialActionRecord';
import type { ProvenanceMap } from './generated/ProvenanceMap';
import type { SideEffectClass } from './generated/SideEffectClass';
import type { Source } from './generated/Source';
import type { TraceListResponse } from './generated/TraceListResponse';
import type { ToolMetadataEntry } from './generated/ToolMetadataEntry';
import type { ToolMetadataListResponse } from './generated/ToolMetadataListResponse';
import type { UpdateRunRequest } from './generated/UpdateRunRequest';
import type { UpsertToolMetadataRequest } from './generated/UpsertToolMetadataRequest';
import { Decode, SdkError, Transport, fromResponse, parseRetryAfter } from './errors';
import { DEFAULT_RETRY, type RetryConfig, nextDelay } from './retry';

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
  getStore(): ActiveRunContext | undefined;
  run<T>(store: ActiveRunContext, callback: () => Promise<T>): Promise<T>;
}

function browserRunContext(): RunContextStore {
  let current: ActiveRunContext | undefined;
  return {
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
          AsyncLocalStorage: new () => RunContextStore;
        };
        return new mod.AsyncLocalStorage();
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

export interface GuardToolCallOptions {
  agentId: string;
  operation: string;
  parameters?: Record<string, unknown>;
  sideEffect?: SideEffectClass;
  sources?: Source[];
  provenance?: ProvenanceMap;
  context?: Record<string, unknown> | null;
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
  async submitEvent(event: GuardEvent, signal?: AbortSignal): Promise<Decision> {
    const body = await this.withActiveContext(event);
    return this.withRetry(
      (signal) =>
        this.sendJson<Decision>(
          '/v1/events',
          {
            method: 'POST',
            body: JSON.stringify(body),
          },
          signal,
        ),
      signal,
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

  async guardToolCall(opts: GuardToolCallOptions, signal?: AbortSignal): Promise<Decision> {
    return this.submitEvent(
      {
        kind: 'tool.call.proposed',
        principal: {
          workspace_id: '',
          environment_id: '',
          agent_id: opts.agentId,
        },
        action: {
          operation: opts.operation,
          parameters: opts.parameters ?? {},
          ...(opts.sideEffect ? { side_effect: opts.sideEffect } : {}),
        },
        sources: opts.sources ?? [],
        provenance: opts.provenance ?? {},
        context: opts.context ?? null,
      },
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
            body: JSON.stringify(req),
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

  async getFinancialAction(
    actionId: string,
    signal?: AbortSignal,
  ): Promise<FinancialActionRecord> {
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

  async createMandate(
    req: CreateFinancialMandateRequest,
    signal?: AbortSignal,
  ): Promise<FinancialMandate> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialMandate>(
          '/v1/financial/mandates',
          {
            method: 'POST',
            body: JSON.stringify(req),
          },
          signal,
        ),
      signal,
    );
  }

  async listMandates(signal?: AbortSignal): Promise<FinancialMandateListResponse> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialMandateListResponse>(
          '/v1/financial/mandates',
          { method: 'GET' },
          signal,
        ),
      signal,
    );
  }

  async revokeMandate(mandateId: string, signal?: AbortSignal): Promise<FinancialMandate> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialMandate>(
          `/v1/financial/mandates/${encodeURIComponent(mandateId)}/revoke`,
          { method: 'POST', body: JSON.stringify({}) },
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

  async approveAction(actionId: string, signal?: AbortSignal): Promise<FinancialActionRecord> {
    return this.transitionFinancialAction(actionId, 'approve', signal);
  }

  async denyAction(actionId: string, signal?: AbortSignal): Promise<FinancialActionRecord> {
    return this.transitionFinancialAction(actionId, 'deny', signal);
  }

  async executeAction(actionId: string, signal?: AbortSignal): Promise<FinancialActionRecord> {
    return this.transitionFinancialAction(actionId, 'execute', signal);
  }

  private async transitionFinancialAction(
    actionId: string,
    transition: 'approve' | 'deny' | 'execute',
    signal?: AbortSignal,
  ): Promise<FinancialActionRecord> {
    return this.withRetry(
      (signal) =>
        this.sendJson<FinancialActionRecord>(
          `/v1/financial/actions/${encodeURIComponent(actionId)}/${transition}`,
          { method: 'POST', body: JSON.stringify({}) },
          signal,
        ),
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
    if (
      !principal.run_event_id &&
      principal.run_id === context.runId &&
      context.runEventId
    ) {
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

  async updateRun(
    runId: string,
    req: UpdateRunRequest,
    signal?: AbortSignal,
  ): Promise<RunSummary> {
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

  async listTraces(options: ListTracesOptions = {}, signal?: AbortSignal): Promise<TraceListResponse> {
    const query = new URLSearchParams();
    if (options.limit !== undefined) query.set('limit', String(options.limit));
    if (options.sessionId !== undefined) query.set('session_id', options.sessionId);
    const suffix = query.size > 0 ? `?${query.toString()}` : '';
    return this.withRetry(
      (signal) => this.sendJson<TraceListResponse>(`/v1/traces${suffix}`, { method: 'GET' }, signal),
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

  async listPolicies(signal?: AbortSignal): Promise<PolicyListResponse> {
    return this.withRetry(
      (signal) => this.sendJson<PolicyListResponse>('/v1/policies', { method: 'GET' }, signal),
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
