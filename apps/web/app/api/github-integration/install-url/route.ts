import { NextResponse } from 'next/server';
import { z } from 'zod';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

const bodySchema = z.object({
  redirect_path: z.string().optional(),
});

export async function POST(req: Request): Promise<NextResponse> {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    body = {};
  }
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: 'invalid install-url request' }, { status: 400 });
  }
  return proxyRustJson(req, '/v1/github-integration/install-url', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(parsed.data),
  });
}
