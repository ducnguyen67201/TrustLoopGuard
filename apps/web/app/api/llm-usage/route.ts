import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';

// Thin proxy: browser → here → Rust GET /v1/llm-usage, scoped to the signed-in
// workspace. `proxyRustJson` does not forward query strings itself, so the
// filter/rollup params (group_by, principal_id, model, start, end) are carried
// over via forwardedQuery — same pattern as the traces and analytics proxies.
export const runtime = 'nodejs';

export async function GET(req: Request) {
  const url = new URL(req.url);
  return proxyRustJson(req, `/v1/llm-usage${forwardedQuery(url.searchParams)}`);
}
