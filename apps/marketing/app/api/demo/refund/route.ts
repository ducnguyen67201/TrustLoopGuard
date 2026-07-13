import { NextResponse } from 'next/server';
import { ZodError } from 'zod';
import {
  parseRefundDemoPrompt,
  refundDemoProxySecret,
  refundDemoServiceUrl,
  sanitizeRefundDemoResponse,
} from '@/app/demo/contract';

const RATE_LIMIT_WINDOW_MS = 10 * 60 * 1_000;
const RATE_LIMIT_MAX = 4;
const RATE_LIMIT_MAX_ENTRIES = 10_000;
const UPSTREAM_TIMEOUT_MS = 45_000;

// Per-visitor UX throttle. The authenticated refund service owns the central launch budget.
const hits = new Map<string, { count: number; resetAt: number }>();

export async function POST(request: Request) {
  if (isRateLimited(request)) {
    return NextResponse.json(
      { error: 'Demo limit reached. Try again in a few minutes.' },
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
    const upstream = await fetch(`${refundDemoServiceUrl()}/chat`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${refundDemoProxySecret()}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify({ prompt }),
      cache: 'no-store',
      signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
    });
    const payload: unknown = await upstream.json().catch(() => ({}));
    if (!upstream.ok) {
      return NextResponse.json(
        { error: 'The refund workflow failed safely. No refund was executed.' },
        { status: upstream.status },
      );
    }

    let response;
    try {
      response = sanitizeRefundDemoResponse(payload);
    } catch (error) {
      console.error('refund demo upstream contract failed', safeErrorForLog(error));
      return NextResponse.json(
        { error: 'The refund workflow returned an invalid response. No refund was executed.' },
        { status: 502 },
      );
    }

    return NextResponse.json(response, {
      headers: { 'cache-control': 'no-store' },
    });
  } catch (error) {
    console.error('refund demo request failed', safeErrorForLog(error));
    return NextResponse.json(
      { error: 'The live demo is temporarily unavailable. No refund was executed.' },
      { status: 503 },
    );
  }
}

function isRateLimited(request: Request): boolean {
  const now = Date.now();
  const key = clientAddress(request);
  const current = hits.get(key);
  if (current === undefined || current.resetAt <= now) {
    pruneExpiredHits(now);
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

function pruneExpiredHits(now: number): void {
  for (const [key, entry] of hits) {
    if (entry.resetAt <= now) hits.delete(key);
  }
}

function safeErrorForLog(error: unknown): string {
  if (!(error instanceof Error)) return 'unknown error';
  return `${error.name}: ${error.message}`.slice(0, 500);
}
