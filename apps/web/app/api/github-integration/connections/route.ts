import { NextResponse } from 'next/server';
import { z } from 'zod';

import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

const bodySchema = z.object({
  repository_id: z.string().regex(/^\d+$/),
  root_path: z.string().max(400),
  agent_id: z.string().min(1).max(200),
  environment_id: z.string().min(1).max(200),
});

export async function GET(req: Request) {
  const url = new URL(req.url);
  return proxyRustJson(req, `/v1/github-integration/connections${forwardedQuery(url.searchParams)}`);
}

export async function POST(req: Request): Promise<NextResponse> {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'request body must be JSON' }, { status: 400 });
  }
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid connection request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }
  return proxyRustJson(req, '/v1/github-integration/connections', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(parsed.data),
  });
}
