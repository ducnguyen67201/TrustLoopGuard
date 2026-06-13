import { NextResponse } from 'next/server';
import { z } from 'zod';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

// Snake_case wire body forwarded to Rust (CreateReportRequest).
const createReportBodySchema = z.object({
  job_id: z.string().min(1),
  compare_job_id: z.string().min(1).optional(),
  ttl_days: z.number().int().positive().optional(),
});

export async function POST(req: Request): Promise<NextResponse> {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'request body must be JSON' }, { status: 400 });
  }

  const parsed = createReportBodySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid report request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }

  return proxyRustJson(req, '/v1/redteam/reports', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(parsed.data),
  });
}
