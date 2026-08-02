import { Client, type ClientOptions } from '@featherlane-ai/sdk';

const DEFAULT_FEATHERLANE_AI_URL = 'http://127.0.0.1:8080';

export type ClientEnv = Partial<Record<'FEATHERLANE_AI_URL' | 'FEATHERLANE_AI_API_KEY', string>>;

export function readClientOptions(env: ClientEnv): ClientOptions {
  const apiKey = env.FEATHERLANE_AI_API_KEY?.trim();
  if (!apiKey) {
    throw new Error('FEATHERLANE_AI_API_KEY is required');
  }
  return {
    baseUrl: env.FEATHERLANE_AI_URL?.trim() || DEFAULT_FEATHERLANE_AI_URL,
    apiKey,
  };
}

export function createFeatherlaneAIClient(env: ClientEnv = process.env): Client {
  return new Client(readClientOptions(env));
}
