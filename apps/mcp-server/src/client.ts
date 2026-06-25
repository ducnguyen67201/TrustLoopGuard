import { Client, type ClientOptions } from '@trustloopguard/sdk';

const DEFAULT_TLG_URL = 'http://127.0.0.1:8080';

export type ClientEnv = Partial<Record<'TLG_URL' | 'TLG_API_KEY', string>>;

export function readClientOptions(env: ClientEnv): ClientOptions {
  const apiKey = env.TLG_API_KEY?.trim();
  if (!apiKey) {
    throw new Error('TLG_API_KEY is required');
  }
  return {
    baseUrl: env.TLG_URL?.trim() || DEFAULT_TLG_URL,
    apiKey,
  };
}

export function createTrustLoopClient(env: ClientEnv = process.env): Client {
  return new Client(readClientOptions(env));
}
