// Thin HTTP client. Mirrors the `Guard.check(draft, ctx)` plugin contract.

import type { CheckRequest } from './generated/CheckRequest';
import type { Decision } from './generated/Decision';
import { Decode, Transport, fromResponse, parseRetryAfter } from './errors';

export interface ClientOptions {
  baseUrl: string;
  apiKey?: string;
  fetchImpl?: typeof fetch;
}

export class Client {
  private readonly baseUrl: string;
  private readonly apiKey: string | undefined;
  private readonly fetchImpl: typeof fetch;

  constructor(opts: ClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/$/, '');
    this.apiKey = opts.apiKey;
    this.fetchImpl = opts.fetchImpl ?? fetch;
  }

  async check(req: CheckRequest, signal?: AbortSignal): Promise<Decision> {
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
