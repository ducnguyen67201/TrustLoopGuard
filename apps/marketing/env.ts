import { createEnv } from '@t3-oss/env-nextjs';
import { z } from 'zod';

const publicUrl = z.url();
const gtmId = z.string().regex(/^GTM-[A-Z0-9]+$/, 'Must look like GTM-XXXXXXX');

export const env = createEnv({
  client: {
    NEXT_PUBLIC_BOOK_MEETING_URL: publicUrl.default(
      'https://calendar.app.google/aQc6ws3pDWpUKFzS9',
    ),
    NEXT_PUBLIC_DOCS_URL: publicUrl.default('https://docs.gettrustloop.app/'),
    NEXT_PUBLIC_SITE_URL: publicUrl.default('https://gettrustloop.app'),
    NEXT_PUBLIC_GTM_ID: gtmId.optional(),
  },
  runtimeEnv: {
    NEXT_PUBLIC_BOOK_MEETING_URL: process.env['NEXT_PUBLIC_BOOK_MEETING_URL'],
    NEXT_PUBLIC_DOCS_URL: process.env['NEXT_PUBLIC_DOCS_URL'],
    NEXT_PUBLIC_SITE_URL: process.env['NEXT_PUBLIC_SITE_URL'],
    NEXT_PUBLIC_GTM_ID: process.env['NEXT_PUBLIC_GTM_ID'],
  },
  emptyStringAsUndefined: true,
  skipValidation: process.env['SKIP_ENV_VALIDATION'] === 'true',
});
