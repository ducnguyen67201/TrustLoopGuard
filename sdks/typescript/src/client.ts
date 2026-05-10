// Thin HTTP client. Mirrors the `Guard.check(draft, ctx)` plugin contract.

import type { CheckRequest } from "./generated/CheckRequest";
import type { Decision } from "./generated/Decision";

export interface ClientOptions {
  baseUrl: string;
  apiKey?: string;
  fetchImpl?: typeof fetch;
}

export class Client {
  private readonly baseUrl: string;
  private readonly apiKey?: string;
  private readonly fetchImpl: typeof fetch;

  constructor(opts: ClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/$/, "");
    this.apiKey = opts.apiKey;
    this.fetchImpl = opts.fetchImpl ?? fetch;
  }

  async check(req: CheckRequest, signal?: AbortSignal): Promise<Decision> {
    const headers: Record<string, string> = {
      "content-type": "application/json",
    };
    if (this.apiKey) headers["authorization"] = `Bearer ${this.apiKey}`;

    const res = await this.fetchImpl(`${this.baseUrl}/v1/check`, {
      method: "POST",
      headers,
      body: JSON.stringify(req),
      signal,
    });
    if (!res.ok) {
      throw new Error(`tl-server returned ${res.status}`);
    }
    return (await res.json()) as Decision;
  }
}
