import { NextResponse } from 'next/server';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

/**
 * `POST /api/redteam/jobs/{id}/harden` — synthesize + verify guardrails from a
 * job's landed attacks. Thin proxy: synthesis and verification are owned by Rust.
 */
export async function POST(req: Request, context: RouteContext): Promise<NextResponse> {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/redteam/jobs/${encodeURIComponent(id)}/harden`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: await req.text(),
  });
}
