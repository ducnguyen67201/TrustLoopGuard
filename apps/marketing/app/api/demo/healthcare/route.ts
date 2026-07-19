import {
  HealthcareDemoBudgetExceededError,
  readHostedHealthcareDemoPolicies,
  runHostedHealthcareDemo,
  type HostedHealthcareDemoResponse,
  type HostedHealthcarePolicyInventoryResponse,
} from '@trustloopguard/demo/healthcare-agent/hosted';
import { NextResponse } from 'next/server';
import { ZodError } from 'zod';

import {
  parseHealthcareDemoRequest,
  sanitizeHealthcareDemoResponse,
  sanitizeHealthcarePolicyInventory,
  type HealthcareDemoRequest,
} from '@/app/demo/healthcare/contract';

export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

const RATE_LIMIT_WINDOW_MS = 24 * 60 * 60 * 1_000;
const RATE_LIMIT_MAX = 10;
const RATE_LIMIT_MAX_ENTRIES = 10_000;

interface HealthcareDemoHandlersDependencies {
  runWorkflow: (request: HealthcareDemoRequest) => Promise<HostedHealthcareDemoResponse>;
  readPolicies: () => Promise<HostedHealthcarePolicyInventoryResponse>;
}

export function createHealthcareDemoHandlers(
  dependencies: HealthcareDemoHandlersDependencies = {
    runWorkflow: runHostedHealthcareDemo,
    readPolicies: readHostedHealthcareDemoPolicies,
  },
) {
  // These fixed-window controls are intentionally process-local. The central
  // hosted budget still caps model calls across visitors in this process.
  const hits = new Map<string, { count: number; resetAt: number }>();

  async function GET() {
    try {
      const inventory = await dependencies.readPolicies();
      return NextResponse.json(sanitizeHealthcarePolicyInventory(inventory), {
        headers: { 'cache-control': 'no-store' },
      });
    } catch (error) {
      console.error(
        'healthcare demo policy inventory failed',
        error instanceof Error
          ? `${error.name}: ${error.message}`.slice(0, 500)
          : 'unknown error',
      );
      return NextResponse.json(
        { error: 'The policy inventory is temporarily unavailable.' },
        { status: 503 },
      );
    }
  }

  async function POST(request: Request) {
    if (isRateLimited(request, hits)) {
      return NextResponse.json(
        { error: 'Daily healthcare demo limit reached. Try again tomorrow.' },
        { status: 429 },
      );
    }

    let demoRequest: HealthcareDemoRequest;
    try {
      demoRequest = parseHealthcareDemoRequest(await request.json());
    } catch (error) {
      if (error instanceof ZodError) {
        return NextResponse.json(
          { error: error.issues[0]?.message ?? 'Invalid healthcare demo request.' },
          { status: 400 },
        );
      }
      return NextResponse.json({ error: 'Request body must be valid JSON.' }, { status: 400 });
    }

    try {
      const payload = await dependencies.runWorkflow(demoRequest);
      try {
        const response = sanitizeHealthcareDemoResponse(payload);
        return NextResponse.json(response, {
          headers: { 'cache-control': 'no-store' },
        });
      } catch (error) {
        console.error(
          'healthcare demo workflow contract failed',
          error instanceof Error
            ? `${error.name}: ${error.message}`.slice(0, 500)
            : 'unknown error',
        );
        return NextResponse.json(
          { error: 'The protected healthcare workflow returned an invalid response.' },
          { status: 502 },
        );
      }
    } catch (error) {
      if (error instanceof HealthcareDemoBudgetExceededError) {
        return NextResponse.json(
          { error: 'Healthcare demo budget reached. Try again later.' },
          { status: 429 },
        );
      }
      console.error(
        'healthcare demo request failed',
        error instanceof Error
          ? `${error.name}: ${error.message}`.slice(0, 500)
          : 'unknown error',
      );
      return NextResponse.json(
        { error: 'The protected healthcare demo is temporarily unavailable.' },
        { status: 503 },
      );
    }
  }

  return { GET, POST };
}

const handlers = createHealthcareDemoHandlers();
export const GET = handlers.GET;
export const POST = handlers.POST;

function isRateLimited(
  request: Request,
  hits: Map<string, { count: number; resetAt: number }>,
): boolean {
  if (process.env['NODE_ENV'] !== 'production') return false;

  const now = Date.now();
  const key = clientAddress(request);
  const current = hits.get(key);
  if (current === undefined || current.resetAt <= now) {
    pruneExpiredHits(hits, now);
    while (hits.size >= RATE_LIMIT_MAX_ENTRIES) {
      const oldest = hits.keys().next().value as string | undefined;
      if (oldest === undefined) break;
      hits.delete(oldest);
    }
    hits.set(key, { count: 1, resetAt: now + RATE_LIMIT_WINDOW_MS });
    return false;
  }
  current.count += 1;
  return current.count > RATE_LIMIT_MAX;
}

function clientAddress(request: Request): string {
  const platformAddress = request.headers
    .get('x-vercel-forwarded-for')
    ?.split(',')[0]
    ?.trim();
  const forwarded = request.headers.get('x-forwarded-for')?.split(',')[0]?.trim();
  return (
    platformAddress ||
    request.headers.get('cf-connecting-ip') ||
    request.headers.get('x-real-ip') ||
    forwarded ||
    'local'
  );
}

function pruneExpiredHits(
  hits: Map<string, { count: number; resetAt: number }>,
  now: number,
): void {
  for (const [key, entry] of hits) {
    if (entry.resetAt <= now) hits.delete(key);
  }
}
