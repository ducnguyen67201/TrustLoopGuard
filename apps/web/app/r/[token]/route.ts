/**
 * Public, unauthenticated PDF for a shared vulnerability report.
 *
 * The token is the bearer capability (minted by Rust), so this route needs no
 * session: a prospect can open the link directly. It fetches the Rust-owned
 * report payload, renders the branded `@react-pdf/renderer` document server-side,
 * and streams it as a PDF. Node runtime — react-pdf needs Node, not Edge.
 */
import { renderToBuffer } from '@react-pdf/renderer';

import {
  fetchPublicReport,
  ReportNotFoundError,
  ReportRateLimitedError,
  type RedteamReportPayload,
} from '@/lib/redteam-report';

import { ReportDocument } from './report-document';

export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

interface RouteContext {
  params: Promise<{ token: string }>;
}

export async function GET(_req: Request, context: RouteContext): Promise<Response> {
  const { token } = await context.params;

  let payload: RedteamReportPayload;
  try {
    payload = await fetchPublicReport(token);
  } catch (err) {
    if (err instanceof ReportNotFoundError) {
      return new Response('Report not found or expired.', { status: 404 });
    }
    if (err instanceof ReportRateLimitedError) {
      return new Response('Too many requests — try again shortly.', {
        status: 429,
        headers: { 'Retry-After': '60' },
      });
    }
    return new Response('Could not load report.', { status: 502 });
  }

  // Call the component directly: server-side, no hooks, and the returned
  // `<Document>` element is what `renderToBuffer` expects.
  const buffer = await renderToBuffer(ReportDocument({ payload }));
  return new Response(new Uint8Array(buffer), {
    headers: {
      'Content-Type': 'application/pdf',
      'Content-Disposition': `inline; filename="trustloopguard-report-${token}.pdf"`,
      // A shared report can contain attack evidence — keep it out of indexes/caches.
      'X-Robots-Tag': 'noindex, nofollow',
      'Cache-Control': 'private, no-store',
    },
  });
}
