import posthog from 'posthog-js';

import { env } from '@/env';
import { initializeMarketingPostHog } from '@/lib/posthog';

initializeMarketingPostHog(posthog, {
  projectToken: env.NEXT_PUBLIC_POSTHOG_PROJECT_TOKEN,
  host: env.NEXT_PUBLIC_POSTHOG_HOST,
});
