import { NextResponse } from 'next/server';

import { errorResponse } from '@/app/api/_shared';
import { env } from '@/env';
import { isAllowedAgentTargetUrl, redteamRunRequestSchema } from '@/lib/arena-redteam';

export const runtime = 'nodejs';

function runnerBaseUrl(): string {
  return env.REDTEAM_RUNNER_URL.replace(/\/$/, '');
}

async function passthrough(response: Response): Promise<NextResponse> {
  const text = await response.text();
  if (text === '') return new NextResponse(null, { status: response.status });
  try {
    return NextResponse.json(JSON.parse(text) as unknown, { status: response.status });
  } catch {
    return new NextResponse(text, { status: response.status });
  }
}

function unreachable(err: unknown): NextResponse {
  // A refused/failed fetch to the local backend surfaces as a TypeError in Node's
  // fetch. Translate it into a clear, actionable 503 instead of a generic 502.
  if (err instanceof TypeError) {
    return NextResponse.json(
      {
        error: `red-team backend not reachable at ${runnerBaseUrl()}. Start the TrustLoopRed backend (uvicorn runner:app --port 8799) or set REDTEAM_RUNNER_URL.`,
      },
      { status: 503 },
    );
  }
  return errorResponse(err);
}

export async function POST(req: Request): Promise<NextResponse> {
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

  try {
    const response = await fetch(`${runnerBaseUrl()}/redteam/run`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(parsed.data),
    });
    return await passthrough(response);
  } catch (err) {
    return unreachable(err);
  }
}

export async function GET(req: Request): Promise<NextResponse> {
  const runId = new URL(req.url).searchParams.get('runId');
  if (runId === null || runId === '') {
    return NextResponse.json({ error: 'runId query parameter is required' }, { status: 400 });
  }

  try {
    const response = await fetch(`${runnerBaseUrl()}/redteam/runs/${encodeURIComponent(runId)}`);
    return await passthrough(response);
  } catch (err) {
    return unreachable(err);
  }
}
