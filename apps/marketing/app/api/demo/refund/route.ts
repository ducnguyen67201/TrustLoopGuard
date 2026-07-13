import { NextResponse } from 'next/server';
import { ZodError } from 'zod';
import {
  parseRefundDemoPrompt,
  refundDemoServiceUrl,
  sanitizeRefundDemoResponse,
} from '@/app/demo/contract';

const RATE_LIMIT_WINDOW_MS = 10 * 60 * 1_000;
const RATE_LIMIT_MAX = 4;
const UPSTREAM_TIMEOUT_MS = 45_000;

// Public-demo throttle. A shared store should replace this if the route runs on multiple instances.
const hits = new Map<string, { count: number; resetAt: number }>();

export async function POST(request: Request) {
  if (isRateLimited(request)) {
    return NextResponse.json(
      { error: 'Demo limit reached. Try again in a few minutes.' },
      { status: 429 },
    );
  }

  try {
    const body = await request.json();
    const prompt = parseRefundDemoPrompt(body);
    const upstream = await fetch(`${refundDemoServiceUrl()}/chat`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
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

    return NextResponse.json(sanitizeRefundDemoResponse(payload), {
      headers: { 'cache-control': 'no-store' },
    });
  } catch (error) {
    if (error instanceof ZodError) {
      return NextResponse.json(
        { error: error.issues[0]?.message ?? 'Invalid demo request.' },
        { status: 400 },
      );
    }
    console.error('refund demo request failed', safeErrorForLog(error));
    return NextResponse.json(
      { error: 'The live demo is temporarily unavailable. No refund was executed.' },
      { status: 503 },
    );
  }
}

function isRateLimited(request: Request): boolean {
  const now = Date.now();
  const forwarded = request.headers.get('x-forwarded-for')?.split(',')[0]?.trim();
  const key = forwarded || request.headers.get('x-real-ip') || 'local';
  const current = hits.get(key);
  if (current === undefined || current.resetAt <= now) {
    hits.set(key, { count: 1, resetAt: now + RATE_LIMIT_WINDOW_MS });
    return false;
  }
  current.count += 1;
  return current.count > RATE_LIMIT_MAX;
}

function safeErrorForLog(error: unknown): string {
  if (!(error instanceof Error)) return 'unknown error';
  return `${error.name}: ${error.message}`.slice(0, 500);
}
