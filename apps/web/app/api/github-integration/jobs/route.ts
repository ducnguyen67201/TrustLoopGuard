import { NextResponse } from 'next/server';
import { z } from 'zod';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

const bodySchema = z.object({
  connection_id: z.string().uuid(),
  risk_statement: z.string().min(20).max(1200),
  source_processing_consent: z.boolean(),
});

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
      { error: 'invalid job request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }
  return proxyRustJson(req, '/v1/github-integration/jobs', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(parsed.data),
  });
}
