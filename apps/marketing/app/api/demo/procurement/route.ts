import {
  readHostedProcurementDemoPolicies,
  readHostedProcurementDemoPolicyPreview,
  runHostedProcurementDemo,
  type HostedProcurementDemoResponse,
  type HostedProcurementPolicyInventoryResponse,
} from '@featherlane-ai/demo/procurement-agent/hosted';
import { NextResponse } from 'next/server';
import { ZodError } from 'zod';

import {
  parseProcurementDemoRequest,
  sanitizeProcurementDemoResponse,
  sanitizeProcurementPolicyInventory,
  type JsonValue,
  type ProcurementPolicyId,
} from '@/app/demo/procurement/contract';

export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

const RATE_LIMIT_WINDOW_MS = 24 * 60 * 60 * 1_000;
const RATE_LIMIT_MAX = 10;
const RATE_LIMIT_MAX_ENTRIES = 10_000;

interface ProcurementDemoHandlersDependencies {
  runWorkflow: (
    prompt: string,
    activePolicyIds: readonly ProcurementPolicyId[],
  ) => Promise<HostedProcurementDemoResponse | JsonValue>;
  readPolicies?: () => Promise<HostedProcurementPolicyInventoryResponse>;
}

export function createProcurementDemoHandlers(
  dependencies: ProcurementDemoHandlersDependencies = {
    runWorkflow: runHostedProcurementDemo,
    readPolicies: readHostedProcurementDemoPolicies,
  },
) {
  // The public Marketing deployment uses one replica, so this fixed-window
  // visitor control is intentionally process-local.
  const hits = new Map<string, { count: number; resetAt: number }>();

  async function GET() {
    try {
      const inventory = await (
        dependencies.readPolicies ?? readHostedProcurementDemoPolicies
      )();
      return NextResponse.json(sanitizeProcurementPolicyInventory(inventory), {
        headers: { 'cache-control': 'no-store' },
      });
    } catch (error) {
      console.warn(
        'procurement demo policy inventory unavailable; showing template preview',
        safeErrorForLog(error instanceof Error ? error : undefined),
      );
      return NextResponse.json(
        sanitizeProcurementPolicyInventory(readHostedProcurementDemoPolicyPreview()),
        { headers: { 'cache-control': 'no-store' } },
      );
    }
  }

  async function POST(request: Request) {
    if (isRateLimited(request, hits)) {
      return NextResponse.json(
        { error: 'Daily demo limit reached. Try again tomorrow.' },
        { status: 429 },
      );
    }

    let parsedRequest;
    try {
      const body: JsonValue = await request.json();
      parsedRequest = parseProcurementDemoRequest(body);
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
      const payload = await dependencies.runWorkflow(
        parsedRequest.prompt,
        parsedRequest.activePolicyIds,
      );
      try {
        const serializablePayload: JsonValue = JSON.parse(JSON.stringify(payload));
        const response = sanitizeProcurementDemoResponse(serializablePayload);
        return NextResponse.json(response, {
          headers: { 'cache-control': 'no-store' },
        });
      } catch (error) {
        console.error(
          'procurement demo workflow contract failed',
          safeErrorForLog(error instanceof Error ? error : undefined),
        );
        return NextResponse.json(
          {
            error:
              'The procurement workflow returned an invalid response. No purchase order was submitted.',
          },
          { status: 502 },
        );
      }
    } catch (error) {
      if (error instanceof Error && error.name === 'ProcurementDemoBudgetExceededError') {
        return NextResponse.json(
          { error: 'Demo budget reached. Try again later.' },
          { status: 429 },
        );
      }
      console.error(
        'procurement demo request failed',
        safeErrorForLog(error instanceof Error ? error : undefined),
      );
      return NextResponse.json(
        {
          error:
            'The live procurement demo is temporarily unavailable. No purchase order was submitted.',
        },
        { status: 503 },
      );
    }
  }

  return { GET, POST };
}

const handlers = createProcurementDemoHandlers();
export const GET = handlers.GET;
export const POST = handlers.POST;

function isRateLimited(
  request: Request,
  hits: Map<string, { count: number; resetAt: number }>,
): boolean {
  if (process.env['NODE_ENV'] !== 'production') return false;

  const now = Date.now();
  const key = clientAddress(request);
  const current = hits.get(key);
  if (current === undefined || current.resetAt <= now) {
    pruneExpiredHits(hits, now);
    while (hits.size >= RATE_LIMIT_MAX_ENTRIES) {
      const oldest = hits.keys().next();
      if (oldest.done === true) break;
      hits.delete(oldest.value);
    }
    hits.set(key, { count: 1, resetAt: now + RATE_LIMIT_WINDOW_MS });
    return false;
  }
  current.count += 1;
  return current.count > RATE_LIMIT_MAX;
}

function clientAddress(request: Request): string {
  const platformAddress = request.headers.get('x-vercel-forwarded-for')?.split(',')[0]?.trim();
  const forwarded = request.headers.get('x-forwarded-for')?.split(',')[0]?.trim();
  return (
    platformAddress ||
    request.headers.get('cf-connecting-ip') ||
    request.headers.get('x-real-ip') ||
    forwarded ||
    'local'
  );
}

function pruneExpiredHits(
  hits: Map<string, { count: number; resetAt: number }>,
  now: number,
): void {
  for (const [key, entry] of hits) {
    if (entry.resetAt <= now) hits.delete(key);
  }
}

function safeErrorForLog(error?: Error): string {
  if (error === undefined) return 'unknown error';
  return error.name.slice(0, 100);
}
