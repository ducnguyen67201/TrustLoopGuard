import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';

// Thin proxy: browser → here → Rust GET /v1/traces, scoped to the signed-in
// workspace. Uses the shared proxyRustJson helper (status passthrough + 204
// handling + error mapping). The review-queue page reads recent traces and
// filters escalate/block client-side (the Rust endpoint has no verdict filter).
export const runtime = 'nodejs';

export async function GET(req: Request) {
  const url = new URL(req.url);
  return proxyRustJson(req, `/v1/traces${forwardedQuery(url.searchParams)}`);
}
