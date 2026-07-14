import {
  readHostedRefundDemoStatus,
  runHostedRefundDemo,
} from '@trustloopguard/demo/stripe-refund-agent/hosted';
import { NextResponse } from 'next/server';
import { ZodError } from 'zod';
import {
  parseRefundDemoActionId,
  parseRefundDemoPrompt,
  sanitizeRefundDemoResponse,
  sanitizeRefundDemoStatus,
} from '@/app/demo/contract';

export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

const RATE_LIMIT_WINDOW_MS = 24 * 60 * 60 * 1_000;
const RATE_LIMIT_MAX = 10;
const RATE_LIMIT_MAX_ENTRIES = 10_000;

interface RefundDemoHandlersDependencies {
  runWorkflow: (prompt: string) => Promise<unknown>;
  readStatus: (actionId: string) => Promise<unknown>;
}

export function createRefundDemoHandlers(
  dependencies: RefundDemoHandlersDependencies = {
    runWorkflow: runHostedRefundDemo,
    readStatus: readHostedRefundDemoStatus,
  },
) {
  // These fixed-window controls are intentionally process-local. Railway is
  // configured to run exactly one Marketing replica for the public launch.
  const hits = new Map<string, { count: number; resetAt: number }>();

  async function GET(request: Request) {
    let actionId: string;
    try {
      actionId = parseRefundDemoActionId(new URL(request.url).searchParams.get('actionId'));
    } catch (error) {
      if (error instanceof ZodError) {
        return NextResponse.json({ error: 'A valid action ID is required.' }, { status: 400 });
      }
      return NextResponse.json({ error: 'Invalid status request.' }, { status: 400 });
    }

    try {
      const payload = await dependencies.readStatus(actionId);
      return NextResponse.json(sanitizeRefundDemoStatus(payload), {
        headers: { 'cache-control': 'no-store' },
      });
    } catch (error) {
      console.error('refund demo status failed', safeErrorForLog(error));
      return NextResponse.json(
        { error: 'Action status is temporarily unavailable.' },
        { status: 503 },
      );
    }
  }

  async function POST(request: Request) {
    if (isRateLimited(request, hits)) {
      return NextResponse.json(
        { error: 'Daily demo limit reached. Try again tomorrow.' },
        { status: 429 },
      );
    }

    let prompt: string;
    try {
      prompt = parseRefundDemoPrompt(await request.json());
    } catch (error) {
      if (error instanceof ZodError) {
        return NextResponse.json(
          { error: error.issues[0]?.message ?? 'Invalid demo request.' },
          { status: 400 },
        );
      }
      return NextResponse.json({ error: 'Request body must be valid JSON.' }, { status: 400 });
    }

    try {
      const payload = await dependencies.runWorkflow(prompt);
      let response;
      try {
        response = sanitizeRefundDemoResponse(payload);
      } catch (error) {
        console.error('refund demo workflow contract failed', safeErrorForLog(error));
        return NextResponse.json(
          { error: 'The refund workflow returned an invalid response. No refund was executed.' },
          { status: 502 },
        );
      }

      return NextResponse.json(response, {
        headers: { 'cache-control': 'no-store' },
      });
    } catch (error) {
      if (error instanceof Error && error.name === 'RefundDemoBudgetExceededError') {
        return NextResponse.json(
          { error: 'Demo budget reached. Try again later.' },
          { status: 429 },
        );
      }
      console.error('refund demo request failed', safeErrorForLog(error));
      return NextResponse.json(
        { error: 'The live demo is temporarily unavailable. No refund was executed.' },
        { status: 503 },
      );
    }
  }

  return { GET, POST };
}

const handlers = createRefundDemoHandlers();
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

function safeErrorForLog(error: unknown): string {
  if (!(error instanceof Error)) return 'unknown error';
  return `${error.name}: ${error.message}`.slice(0, 500);
}
