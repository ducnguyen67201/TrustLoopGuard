import { NextResponse } from 'next/server';

// Returns the running server's build stamp so an open browser tab can compare
// it against the id baked into its own bundle and prompt a refresh after a
// deploy. Not a Rust-owned concern — this is a web-deploy artifact only.
export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

export function GET() {
  return NextResponse.json(
    { buildId: process.env['NEXT_PUBLIC_BUILD_ID'] ?? null },
    { headers: { 'Cache-Control': 'no-store' } },
  );
}
