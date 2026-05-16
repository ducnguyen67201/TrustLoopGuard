import { NextResponse } from 'next/server';

import { getServerUrl } from '@/lib/server-url';

export const runtime = 'nodejs';

interface RouteParams {
  params: Promise<{ id: string }>;
}

// Public — no workspace context, no auth header. Just proxies the
// Rust /v1/invites/:id/lookup so the accept page can render before
// the visitor signs in.
export async function GET(_req: Request, { params }: RouteParams) {
  try {
    const { id } = await params;
    const res = await fetch(
      `${getServerUrl()}/v1/invites/${encodeURIComponent(id)}/lookup`,
      { cache: 'no-store' },
    );
    const text = await res.text();
    return new NextResponse(text, {
      status: res.status,
      headers: {
        'Content-Type': res.headers.get('Content-Type') ?? 'application/json',
      },
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
