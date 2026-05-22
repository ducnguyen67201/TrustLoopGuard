import { createEnv } from '@t3-oss/env-nextjs';
import { z } from 'zod';

const githubRepo = z
  .string()
  .regex(/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/, 'Must be in owner/repo format');

const publicUrl = z.string().url();

export const env = createEnv({
  client: {
    NEXT_PUBLIC_GITHUB_REPO: githubRepo,
    NEXT_PUBLIC_BOOK_MEETING_URL: publicUrl,
    NEXT_PUBLIC_DOCS_URL: publicUrl,
  },
  runtimeEnv: {
    NEXT_PUBLIC_GITHUB_REPO: process.env['NEXT_PUBLIC_GITHUB_REPO'],
    NEXT_PUBLIC_BOOK_MEETING_URL: process.env['NEXT_PUBLIC_BOOK_MEETING_URL'],
    NEXT_PUBLIC_DOCS_URL: process.env['NEXT_PUBLIC_DOCS_URL'],
  },
  emptyStringAsUndefined: true,
  skipValidation: process.env['SKIP_ENV_VALIDATION'] === 'true',
});
