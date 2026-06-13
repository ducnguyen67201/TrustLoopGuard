import { NextResponse } from 'next/server';
import { z } from 'zod';

import { proxyRustJson } from '@/app/api/_shared';
import { isAllowedAgentTargetUrl } from '@/lib/arena-redteam';
import { redteamJobProfileSchema, redteamGeneratorSchema } from '@/lib/redteam-jobs';

export const runtime = 'nodejs';

// Snake_case wire body forwarded to the Rust orchestrator (RedteamDispatchRequest).
const dispatchBodySchema = z.object({
  target_url: z.string().url(),
  profile: redteamJobProfileSchema,
  generator: redteamGeneratorSchema.optional(),
  agent_id: z.string().optional(),
});

export async function POST(req: Request): Promise<NextResponse> {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'request body must be JSON' }, { status: 400 });
  }

  const parsed = dispatchBodySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid dispatch request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }

  // SSRF guard: the orchestrator ultimately fetches this target, so only allow
  // loopback agents (deny-by-default; mirrors the arena proxy).
  if (!isAllowedAgentTargetUrl(parsed.data.target_url)) {
    return NextResponse.json(
      { error: 'target_url must target a loopback agent (127.0.0.1 or localhost)' },
      { status: 400 },
    );
  }

  return proxyRustJson(req, '/v1/redteam/dispatch', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(parsed.data),
  });
}
