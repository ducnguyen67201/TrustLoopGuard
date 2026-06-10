// Thin HTTP client. Mirrors the `Guard.check(draft, ctx)` plugin contract.
//
// Retry policy mirrors tl-sdk-rust and the Python SDK — same defaults,
// same `nextDelay` semantics. Voice callers should pass
// `{ ...DEFAULT_RETRY, maxAttempts: 1 }` to opt out.

import type { CheckRequest } from './generated/CheckRequest';
import type { Decision } from './generated/Decision';
import type { GuardEvent } from './generated/GuardEvent';
import type { AgentListResponse } from './generated/AgentListResponse';
import type { AgentProfile } from './generated/AgentProfile';
import type { ApiKeyBatchRevokeResponse } from './generated/ApiKeyBatchRevokeResponse';
import type { GuardrailGenerateResponse } from './generated/GuardrailGenerateResponse';
import type { GuardrailListResponse } from './generated/GuardrailListResponse';
import type { PolicyDocument } from './generated/PolicyDocument';
import type { PolicyBatchSetEnabledResponse } from './generated/PolicyBatchSetEnabledResponse';
import type { PolicyDraftResponse } from './generated/PolicyDraftResponse';
import type { PolicyListResponse } from './generated/PolicyListResponse';
import type { PolicyValidateResponse } from './generated/PolicyValidateResponse';
import type { CreateRunEventRequest } from './generated/CreateRunEventRequest';
import type { CreateRunRequest } from './generated/CreateRunRequest';
import type { RunDetail } from './generated/RunDetail';
import type { RunEventListResponse } from './generated/RunEventListResponse';
import type { RunEventSummary } from './generated/RunEventSummary';
import type { RunListResponse } from './generated/RunListResponse';
import type { RunStatus } from './generated/RunStatus';
import type { RunSummary } from './generated/RunSummary';
import type { TraceListResponse } from './generated/TraceListResponse';
import type { UpdateRunRequest } from './generated/UpdateRunRequest';
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

  async check(req: CheckRequest, signal?: AbortSignal): Promise<Decision> {
    return this.withRetry(
      (signal) =>
        this.sendJson<Decision>(
          '/v1/check',
          {
            method: 'POST',
            body: JSON.stringify(req),
          },
          signal,
        ),
      signal,
    );
  }

  /**
   * Submit a full `GuardEvent` (sources + provenance) for observe-only
   * evidence collection. The returned decision's verdict is always
   * `allow` with an explicit observe-only reason until checker phases
   * ship; do not gate behavior on it yet.
   */
  async submitEvent(event: GuardEvent, signal?: AbortSignal): Promise<Decision> {
    return this.withRetry(
      (signal) =>
        this.sendJson<Decision>(
          '/v1/events',
          {
            method: 'POST',
            body: JSON.stringify(event),
          },
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
