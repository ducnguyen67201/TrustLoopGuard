import 'server-only';
import { Client } from '@trustloopguard/sdk';
import { getServerUrl } from '../server-url';

let cached: Client | null = null;

export function tlClient(): Client {
  if (cached !== null) return cached;
  cached = new Client({
    baseUrl: getServerUrl(),
    fetchImpl: globalThis.fetch.bind(globalThis),
  });
  return cached;
}
