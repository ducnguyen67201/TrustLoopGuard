import { createEnv } from '@t3-oss/env-nextjs';
import { z } from 'zod';

const publicUrl = z.string().url();

export const env = createEnv({
  client: {
    NEXT_PUBLIC_BOOK_MEETING_URL: publicUrl,
  },
  runtimeEnv: {
    NEXT_PUBLIC_BOOK_MEETING_URL: process.env['NEXT_PUBLIC_BOOK_MEETING_URL'],
  },
  emptyStringAsUndefined: true,
  skipValidation: process.env['SKIP_ENV_VALIDATION'] === 'true',
});
