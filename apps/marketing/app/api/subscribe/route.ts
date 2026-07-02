import { NextResponse } from 'next/server';
import postgres from 'postgres';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/* Signups land in the marketing_waitlist table (Railway Postgres via
   DATABASE_URL). WAITLIST_WEBHOOK_URL is an optional extra hop for email
   notifications (Formspree / Apps Script / Slack). At least one must be set. */

const sql = process.env['DATABASE_URL']
  ? postgres(process.env['DATABASE_URL'], { max: 2, connect_timeout: 8 })
  : null;

async function saveToDb(email: string): Promise<boolean> {
  if (!sql) return false;
  await sql`
    INSERT INTO marketing_waitlist (email, source)
    VALUES (${email}, 'gettrustloop.app')
    ON CONFLICT (email) DO NOTHING
  `;
  return true;
}

async function forwardToWebhook(email: string): Promise<boolean> {
  const webhook = process.env['WAITLIST_WEBHOOK_URL'];
  if (!webhook) return false;
  const res = await fetch(webhook, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      text: `waitlist signup: ${email}`,
      email,
      source: 'gettrustloop.app',
      at: new Date().toISOString(),
    }),
  });
  if (!res.ok) throw new Error(`webhook responded ${res.status}`);
  return true;
}

export async function POST(req: Request) {
  const body = await req.json().catch(() => ({}) as Record<string, unknown>);
  const { email, company } = body as { email?: unknown; company?: unknown };

  // honeypot field — bots fill it, humans never see it
  if (company) return NextResponse.json({ ok: true });

  if (typeof email !== 'string' || !EMAIL_RE.test(email) || email.length > 254) {
    return NextResponse.json({ ok: false, error: 'Enter a valid email.' }, { status: 400 });
  }

  const results = await Promise.allSettled([saveToDb(email), forwardToWebhook(email)]);
  const stored = results.some((r) => r.status === 'fulfilled' && r.value);

  for (const r of results) {
    if (r.status === 'rejected') console.error('waitlist sink failed:', r.reason);
  }

  if (!stored) {
    console.error('no waitlist sink succeeded — set DATABASE_URL or WAITLIST_WEBHOOK_URL');
    return NextResponse.json(
      { ok: false, error: 'Signups are not wired up yet — email us instead.' },
      { status: 503 },
    );
  }

  return NextResponse.json({ ok: true });
}
