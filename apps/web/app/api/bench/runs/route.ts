import { NextResponse } from 'next/server';
import { z } from 'zod';

import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';
import { isAllowedAgentTargetUrl } from '@/lib/redteam-core';
import { redteamGeneratorSchema, redteamJobProfileSchema } from '@/lib/redteam-jobs';

export const runtime = 'nodejs';

const createRunBodySchema = z.object({
  raw_target_url: z.string().url(),
  guarded_target_url: z.string().url(),
  profile: redteamJobProfileSchema,
  generator: redteamGeneratorSchema.optional(),
  agent_id: z.string().optional(),
  seed: z.string().optional(),
});

export async function GET(req: Request): Promise<NextResponse> {
  const query = forwardedQuery(new URL(req.url).searchParams);
  return proxyRustJson(req, `/v1/bench/runs${query}`);
}

export async function POST(req: Request): Promise<NextResponse> {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'request body must be JSON' }, { status: 400 });
  }

  const parsed = createRunBodySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid benchmark run request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }

  if (!isAllowedAgentTargetUrl(parsed.data.raw_target_url)) {
    return NextResponse.json(
      { error: 'raw_target_url must target a loopback agent (127.0.0.1 or localhost)' },
      { status: 400 },
    );
  }
  if (!isAllowedAgentTargetUrl(parsed.data.guarded_target_url)) {
    return NextResponse.json(
      { error: 'guarded_target_url must target a loopback agent (127.0.0.1 or localhost)' },
      { status: 400 },
    );
  }

  return proxyRustJson(req, '/v1/bench/runs', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(parsed.data),
  });
}
