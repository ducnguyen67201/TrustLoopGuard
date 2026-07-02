import { NextResponse } from 'next/server';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const RATE_LIMIT_WINDOW_MS = 10 * 60 * 1000;
const RATE_LIMIT_MAX = 5;
const DEFAULT_NOTIFY_EMAIL = 'duc.nguyen67201@gmail.com';

// ponytail: process-local throttle; use Redis/edge config if signup volume matters.
const hits = new Map<string, { count: number; resetAt: number }>();

function isRateLimited(req: Request): boolean {
  const now = Date.now();
  const forwarded = req.headers.get('x-forwarded-for')?.split(',')[0]?.trim();
  const ip = forwarded || req.headers.get('x-real-ip') || 'unknown';
  const current = hits.get(ip);

  if (!current || current.resetAt <= now) {
    hits.set(ip, { count: 1, resetAt: now + RATE_LIMIT_WINDOW_MS });
    return false;
  }

  current.count += 1;
  return current.count > RATE_LIMIT_MAX;
}

async function forwardToWebhook(email: string): Promise<boolean> {
  const webhook = process.env['WAITLIST_WEBHOOK_URL'];
  if (!webhook) return false;
  const secret = process.env['WAITLIST_WEBHOOK_SECRET'];
  const res = await fetch(webhook, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    signal: AbortSignal.timeout(4000),
    body: JSON.stringify({
      text: `waitlist signup: ${email}`,
      email,
      secret,
      source: 'gettrustloop.app',
      at: new Date().toISOString(),
    }),
  });
  if (!res.ok) throw new Error(`webhook responded ${res.status}`);
  const text = (await res.text().catch(() => '')).trim().toLowerCase();
  if (text.includes('"success":"false"') || text.includes('"success":false')) {
    throw new Error(`webhook rejected signup: ${text.slice(0, 200)}`);
  }
  if (['unauthorized', 'bad request', 'mail failed'].includes(text)) {
    throw new Error(`webhook rejected signup: ${text}`);
  }
  return true;
}

async function sendViaResend(email: string): Promise<boolean> {
  const apiKey = process.env['RESEND_API_KEY'];
  if (!apiKey) return false;

  const to = process.env['WAITLIST_NOTIFY_EMAIL'] || DEFAULT_NOTIFY_EMAIL;
  if (!EMAIL_RE.test(to)) throw new Error('WAITLIST_NOTIFY_EMAIL is invalid');

  const res = await fetch('https://api.resend.com/emails', {
    method: 'POST',
    headers: {
      authorization: `Bearer ${apiKey}`,
      'content-type': 'application/json',
      'user-agent': 'trustloopguard-marketing/1.0',
    },
    signal: AbortSignal.timeout(4000),
    body: JSON.stringify({
      from: process.env['WAITLIST_FROM_EMAIL'] || 'TrustLoopGuard <onboarding@resend.dev>',
      to: [to],
      subject: 'TrustLoop waitlist signup',
      text: `waitlist signup: ${email}`,
    }),
  });

  if (!res.ok) {
    throw new Error(
      `resend responded ${res.status}: ${(await res.text().catch(() => '')).slice(0, 200)}`,
    );
  }
  return true;
}

export async function POST(req: Request) {
  const body = await req.json().catch(() => ({}) as Record<string, unknown>);
  const { email, company } = body as { email?: unknown; company?: unknown };

  // honeypot field — bots fill it, humans never see it
  if (company) return NextResponse.json({ ok: true });

  if (isRateLimited(req)) {
    return NextResponse.json(
      { ok: false, error: 'Too many attempts. Try again later.' },
      { status: 429 },
    );
  }

  const normalizedEmail = typeof email === 'string' ? email.trim().toLowerCase() : '';

  if (!EMAIL_RE.test(normalizedEmail) || normalizedEmail.length > 254) {
    return NextResponse.json({ ok: false, error: 'Enter a valid email.' }, { status: 400 });
  }

  const results = await Promise.allSettled([
    forwardToWebhook(normalizedEmail),
    sendViaResend(normalizedEmail),
  ]);
  const stored = results.some((r) => r.status === 'fulfilled' && r.value);

  for (const r of results) {
    if (r.status === 'rejected') console.error('waitlist sink failed:', r.reason);
  }

  if (!stored) {
    console.error('no waitlist sink succeeded — set WAITLIST_WEBHOOK_URL');
    return NextResponse.json(
      { ok: false, error: 'Signups are not wired up yet — email us instead.' },
      { status: 503 },
    );
  }

  return NextResponse.json({ ok: true });
}
