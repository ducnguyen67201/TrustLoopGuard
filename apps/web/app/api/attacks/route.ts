import { NextResponse } from 'next/server';

import { isAllowedAgentTargetUrl } from '@/lib/arena-redteam';
import { attackRunRequestSchema } from '@/lib/attacks';
import { pollRunnerRun, requireWorkspace, startRunnerRun } from '@/lib/server/runner-proxy';

export const runtime = 'nodejs';

export async function POST(req: Request): Promise<NextResponse> {
  const unauthorized = await requireWorkspace(req);
  if (unauthorized) return unauthorized;

  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'request body must be JSON' }, { status: 400 });
  }

  const parsed = attackRunRequestSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid attack request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }

  // SSRF guard: the agent target is fetched server-side by the runner, so only
  // loopback agents are allowed (deny-by-default allowlist).
  if (!isAllowedAgentTargetUrl(parsed.data.targetUrl)) {
    return NextResponse.json(
      { error: 'targetUrl must target a loopback agent (127.0.0.1 or localhost)' },
      { status: 400 },
    );
  }

  return startRunnerRun({ profile: parsed.data.profile, targetUrl: parsed.data.targetUrl });
}

export async function GET(req: Request): Promise<NextResponse> {
  const unauthorized = await requireWorkspace(req);
  if (unauthorized) return unauthorized;

  const runId = new URL(req.url).searchParams.get('runId');
  if (runId === null || runId === '') {
    return NextResponse.json({ error: 'runId query parameter is required' }, { status: 400 });
  }
  return pollRunnerRun(runId);
}
