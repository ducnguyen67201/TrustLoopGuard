// Re-exported for backwards compatibility with existing call sites.
// Prefer `import { env } from '@/env'` directly in new code.

import { env } from '../env';

export function getServerUrl(): string {
  return env.TL_SERVER_URL;
}
