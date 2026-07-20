import { createEnv } from '@t3-oss/env-nextjs';
import { z } from 'zod';

const publicUrl = z.url();
const gtmId = z.string().regex(/^GTM-[A-Z0-9]+$/, 'Must look like GTM-XXXXXXX');
const postHogProjectToken = z
  .string()
  .regex(/^phc_[A-Za-z0-9]+$/, 'Must be a PostHog project token');

export const env = createEnv({
  server: {
    OUTBOUND_DEMO_DATABASE_URL: z.string().url().optional(),
  },
  client: {
    NEXT_PUBLIC_BOOK_MEETING_URL: publicUrl.default(
      'https://calendar.app.google/aQc6ws3pDWpUKFzS9',
    ),
    NEXT_PUBLIC_DOCS_URL: publicUrl.default('https://docs.gettrustloop.app/'),
    NEXT_PUBLIC_SITE_URL: publicUrl.default('https://gettrustloop.app'),
    NEXT_PUBLIC_GTM_ID: gtmId.optional(),
    NEXT_PUBLIC_POSTHOG_PROJECT_TOKEN: postHogProjectToken.optional(),
    NEXT_PUBLIC_POSTHOG_HOST: publicUrl.default('https://us.i.posthog.com'),
  },
  runtimeEnv: {
    OUTBOUND_DEMO_DATABASE_URL: process.env['OUTBOUND_DEMO_DATABASE_URL'],
    NEXT_PUBLIC_BOOK_MEETING_URL: process.env['NEXT_PUBLIC_BOOK_MEETING_URL'],
    NEXT_PUBLIC_DOCS_URL: process.env['NEXT_PUBLIC_DOCS_URL'],
    NEXT_PUBLIC_SITE_URL: process.env['NEXT_PUBLIC_SITE_URL'],
    NEXT_PUBLIC_GTM_ID: process.env['NEXT_PUBLIC_GTM_ID'],
    NEXT_PUBLIC_POSTHOG_PROJECT_TOKEN: process.env['NEXT_PUBLIC_POSTHOG_PROJECT_TOKEN'],
    NEXT_PUBLIC_POSTHOG_HOST: process.env['NEXT_PUBLIC_POSTHOG_HOST'],
  },
  emptyStringAsUndefined: true,
  skipValidation: process.env['SKIP_ENV_VALIDATION'] === 'true',
});
