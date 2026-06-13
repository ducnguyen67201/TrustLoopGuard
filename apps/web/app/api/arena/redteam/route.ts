import { NextResponse } from 'next/server';

import { isAllowedAgentTargetUrl, redteamRunRequestSchema } from '@/lib/arena-redteam';
import { pollArenaRun, requireWorkspace, startArenaRun } from '@/lib/server/arena-redteam-proxy';

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

  const parsed = redteamRunRequestSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid red-team run request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }

  // SSRF guard: only allow loopback agent targets to be forwarded to the backend.
  const targets: ReadonlyArray<readonly [string, string | undefined]> = [
    ['rawUrl', parsed.data.rawUrl],
    ['guardedUrl', parsed.data.guardedUrl],
  ];
  for (const [field, value] of targets) {
    if (value !== undefined && !isAllowedAgentTargetUrl(value)) {
      return NextResponse.json(
        { error: `${field} must target a loopback agent (127.0.0.1 or localhost)` },
        { status: 400 },
      );
    }
  }

  return startArenaRun(parsed.data);
}

export async function GET(req: Request): Promise<NextResponse> {
  const unauthorized = await requireWorkspace(req);
  if (unauthorized) return unauthorized;

  const runId = new URL(req.url).searchParams.get('runId');
  if (runId === null || runId === '') {
    return NextResponse.json({ error: 'runId query parameter is required' }, { status: 400 });
  }
  return pollArenaRun(runId);
}
