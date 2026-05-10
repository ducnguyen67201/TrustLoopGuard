// Thin HTTP client. Mirrors the `Guard.check(draft, ctx)` plugin contract.
//
// Retry policy mirrors tl-sdk-rust and the Python SDK — same defaults,
// same `nextDelay` semantics. Voice callers should pass
// `{ ...DEFAULT_RETRY, maxAttempts: 1 }` to opt out.

import type { CheckRequest } from './generated/CheckRequest';
import type { Decision } from './generated/Decision';
import {
  Decode,
  SdkError,
  Transport,
  fromResponse,
  parseRetryAfter,
} from './errors';
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
    const start = performance.now();
    let attempt = 0;
    while (true) {
      attempt += 1;
      try {
        return await this.sendOnce(req, signal);
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

  private async sendOnce(req: CheckRequest, signal?: AbortSignal): Promise<Decision> {
    const headers: Record<string, string> = {
      'content-type': 'application/json',
    };
    if (this.apiKey !== undefined) {
      headers['authorization'] = `Bearer ${this.apiKey}`;
    }

    const init: RequestInit = {
      method: 'POST',
      headers,
      body: JSON.stringify(req),
    };
    if (signal !== undefined) {
      init.signal = signal;
    }

    let res: Response;
    try {
      res = await this.fetchImpl(`${this.baseUrl}/v1/check`, init);
    } catch (e) {
      throw new Transport(e instanceof Error ? e.message : String(e));
    }

    if (res.ok) {
      try {
        return (await res.json()) as Decision;
      } catch (e) {
        throw new Decode(`failed to parse Decision: ${String(e)}`);
      }
    }

    const retryAfter = parseRetryAfter(res.headers.get('retry-after'));
    const body = await res.text().catch(() => '');
    throw fromResponse(res.status, body, retryAfter);
  }
}
