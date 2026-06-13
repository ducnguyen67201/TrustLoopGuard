/**
 * Server-side proxy for the **arena** (raw-vs-guarded pair) demo.
 *
 * This backs `/api/arena/redteam` only. It forwards an arena comparison run to
 * the standalone TrustLoopRed attack runner (`REDTEAM_RUNNER_URL`) with an auth
 * gate, timeouts, and failure translation, so the route stays thin
 * (auth → validate → SSRF allowlist → forward). Nothing is persisted — the arena
 * is an ephemeral demo.
 *
 * NOT the durable path: the single-target Attacks tab dispatches through the Rust
 * orchestrator instead (`/api/redteam/*` → `/v1/redteam/*`), which owns the job
 * and calls the same runner via its own `RedteamRunnerClient`. Keep the two
 * separate — this file is arena-only.
 *
 * This is a narrow proxy, not an arbitrary-URL fetcher: callers must be
 * authenticated and agent targets are loopback-allowlisted at the route layer.
 */
import { NextResponse } from 'next/server';

import { errorResponse } from '@/app/api/_shared';
import { env } from '@/env';
import { authorizedWorkspaceIdForRequest, WorkspaceAccessError } from '@/lib/server/tl-client';

// Starting a run returns a handle immediately, but the backend may warm its LLM
// engine first; polls are status reads and should always be quick.
const START_RUN_TIMEOUT_MS = 30_000;
const POLL_RUN_TIMEOUT_MS = 10_000;

export function runnerBaseUrl(): string {
  return env.REDTEAM_RUNNER_URL.replace(/\/$/, '');
}

/** Returns a 401/403 response when the request has no authorized workspace, else null. */
export async function requireWorkspace(req: Request): Promise<NextResponse | null> {
  try {
    await authorizedWorkspaceIdForRequest(req);
    return null;
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    return errorResponse(err);
  }
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

function runnerFailure(err: unknown): NextResponse {
  // AbortSignal.timeout rejects the fetch with a DOMException named TimeoutError.
  if ((err instanceof DOMException || err instanceof Error) && err.name === 'TimeoutError') {
    return NextResponse.json(
      { error: `red-team backend at ${runnerBaseUrl()} timed out` },
      { status: 504 },
    );
  }
  // A refused/failed fetch to the local backend surfaces as a TypeError in Node's
  // fetch. Translate it into a clear, actionable 503.
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

/** Forward a validated arena run-start body to the runner. */
export async function startArenaRun(body: unknown): Promise<NextResponse> {
  try {
    const response = await fetch(`${runnerBaseUrl()}/redteam/run`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(START_RUN_TIMEOUT_MS),
    });
    return await passthrough(response);
  } catch (err) {
    return runnerFailure(err);
  }
}

/** Poll an arena run's status + latest report from the runner. */
export async function pollArenaRun(runId: string): Promise<NextResponse> {
  try {
    const response = await fetch(`${runnerBaseUrl()}/redteam/runs/${encodeURIComponent(runId)}`, {
      signal: AbortSignal.timeout(POLL_RUN_TIMEOUT_MS),
    });
    return await passthrough(response);
  } catch (err) {
    return runnerFailure(err);
  }
}
