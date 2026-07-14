import { NextResponse } from 'next/server';
import { z } from 'zod';

import { auth } from '@/auth';
import { RustApiError, rustApiForUser } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

const callbackSchema = z.object({
  code: z.string().min(1),
  state: z.string().min(1),
  installation_id: z.string().regex(/^\d+$/),
  setup_action: z.string().optional(),
});

export async function GET(req: Request): Promise<NextResponse> {
  const url = new URL(req.url);
  const parsed = callbackSchema.safeParse({
    code: url.searchParams.get('code') ?? '',
    state: url.searchParams.get('state') ?? '',
    installation_id: url.searchParams.get('installation_id') ?? '',
    setup_action: url.searchParams.get('setup_action') ?? undefined,
  });
  if (!parsed.success) {
    return redirect(url, 'error', 'invalid_callback');
  }
  const session = await auth();
  const user = session?.user as
    | { id?: string; email?: string | null; tlJwt?: string | null }
    | undefined;
  if (user?.id === undefined || user.id.trim() === '') {
    return redirect(url, 'error', 'auth_required');
  }
  try {
    await rustApiForUser(
      { id: user.id, email: user.email, tlJwt: user.tlJwt },
      '/v1/github-integration/callback',
      {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(parsed.data),
      },
    );
    return redirect(url, 'connected', 'ok');
  } catch (error) {
    const code =
      error instanceof RustApiError && error.status === 403
        ? 'forbidden'
        : error instanceof RustApiError && error.status === 503
          ? 'unavailable'
          : 'callback_failed';
    return redirect(url, 'error', code);
  }
}

function redirect(url: URL, state: 'connected' | 'error', code: string): NextResponse {
  const target = new URL('/agents', url.origin);
  target.searchParams.set('github', state);
  target.searchParams.set('code', code);
  return NextResponse.redirect(target);
}
