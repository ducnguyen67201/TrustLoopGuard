import { Client } from '@featherlane-ai/sdk';

import { CONTEXTUAL_DEMO_API_KEY, SERVER_URL } from '../shared/env';

export function createContextualRuntimeClient(
  options: { serverUrl?: string; runtimeApiKey?: string; fetchImpl?: typeof fetch } = {},
): Client {
  const runtimeApiKey = (options.runtimeApiKey ?? CONTEXTUAL_DEMO_API_KEY)?.trim();
  if (runtimeApiKey === undefined || runtimeApiKey === '') {
    throw new Error('TL_CONTEXTUAL_DEMO_API_KEY is required for the hosted contextual demo');
  }
  if (!runtimeApiKey.startsWith('tl_live_')) {
    throw new Error('TL_CONTEXTUAL_DEMO_API_KEY must be a workspace runtime key');
  }
  return new Client({
    baseUrl: options.serverUrl ?? SERVER_URL,
    apiKey: runtimeApiKey,
    ...(options.fetchImpl === undefined ? {} : { fetchImpl: options.fetchImpl }),
  });
}
