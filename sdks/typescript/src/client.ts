// Thin HTTP client. Mirrors the `Guard.check(draft, ctx)` plugin contract.
//
// Retry policy mirrors tl-sdk-rust and the Python SDK — same defaults,
// same `nextDelay` semantics. Voice callers should pass
// `{ ...DEFAULT_RETRY, maxAttempts: 1 }` to opt out.

import type { CheckRequest } from './generated/CheckRequest';
import type { Decision } from './generated/Decision';
import type { PolicyDocument } from './generated/PolicyDocument';
import type { PolicyDraftResponse } from './generated/PolicyDraftResponse';
import type { PolicyListResponse } from './generated/PolicyListResponse';
import type { PolicyValidateResponse } from './generated/PolicyValidateResponse';
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
    this.fetchImpl = opts.fetchImpl ?? fetch;
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
