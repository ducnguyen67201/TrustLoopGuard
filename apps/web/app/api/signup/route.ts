import { NextResponse } from 'next/server';

import { isCredentialsAuthEnabled } from '@/lib/auth-capabilities';
import { getServerUrl } from '@/lib/server-url';

export const runtime = 'nodejs';

// Same-origin proxy for the signup form. Browsers can't POST cross-origin
// to tl-server (no CORS layer), so the dashboard hits this route and we
// forward to Rust server-side.
export async function POST(req: Request) {
  if (!isCredentialsAuthEnabled()) {
    return NextResponse.json(
      { message: 'Username sign-up is disabled for this deployment.' },
      { status: 404 },
    );
  }

  const body = await req.text();
  try {
    const res = await fetch(`${getServerUrl()}/v1/auth/signup`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
      cache: 'no-store',
    });
    const text = await res.text();
    return new NextResponse(text, {
      status: res.status,
      headers: { 'Content-Type': res.headers.get('Content-Type') ?? 'application/json' },
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ message }, { status: 502 });
  }
}
