import { isContextualScenarioId } from '@trustloopguard/demo/contextual-agent/config';
import {
  ContextualDemoBudgetExceededError,
  readHostedContextualDemoPolicies,
  runHostedContextualDemo,
  type HostedContextualDemoResponse,
  type HostedContextualPolicyInventoryResponse,
} from '@trustloopguard/demo/contextual-agent/hosted';
import { NextResponse } from 'next/server';
import { ZodError } from 'zod';

import {
  parseContextualDemoRequest,
  sanitizeContextualDemoResponse,
  sanitizeContextualPolicyInventory,
  type ContextualDemoRequest,
} from '@/app/demo/contextual-contract';
import type { OutboundDemoProfile } from '@/app/demo/company-profile';
import { getContextualDemoProfile } from '@/lib/server/outbound-demo-profile-store';

export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

const RATE_LIMIT_WINDOW_MS = 24 * 60 * 60 * 1_000;
const RATE_LIMIT_MAX = 10;
const RATE_LIMIT_MAX_ENTRIES = 10_000;

type RouteContext = { params: Promise<{ category: string }> };

export interface ContextualDemoHandlersDependencies {
  getProfile: (category: string) => Promise<OutboundDemoProfile | null>;
  runWorkflow: typeof runHostedContextualDemo;
  readPolicies: typeof readHostedContextualDemoPolicies;
}

export function createContextualDemoHandlers(
  dependencies: ContextualDemoHandlersDependencies = {
    getProfile: getContextualDemoProfile,
    runWorkflow: runHostedContextualDemo,
    readPolicies: readHostedContextualDemoPolicies,
  },
) {
  const hits = new Map<string, { count: number; resetAt: number }>();

  async function GET(_request: Request, context: RouteContext) {
    const resolved = await resolveProfile(context, dependencies.getProfile);
    if (resolved === null) {
      return NextResponse.json({ error: 'Contextual demo not found.' }, { status: 404 });
    }

    try {
      const inventory = await dependencies.readPolicies(resolved.scenarioId);
      return NextResponse.json(sanitizeContextualPolicyInventory(inventory), {
        headers: { 'cache-control': 'no-store' },
      });
    } catch (error) {
      console.warn(
        'contextual demo policy inventory unavailable',
        safeErrorForLog(error instanceof Error ? error : undefined),
      );
      return NextResponse.json(
        { error: 'The contextual policy registry is temporarily unavailable.' },
        { status: 503, headers: { 'cache-control': 'no-store' } },
      );
    }
  }

  async function POST(request: Request, context: RouteContext) {
    if (isRateLimited(request, hits)) {
      return NextResponse.json(
        { error: 'Daily contextual demo limit reached. Try again tomorrow.' },
        { status: 429 },
      );
    }

    let clientRequest: ContextualDemoRequest;
    try {
      clientRequest = parseContextualDemoRequest(await request.json());
    } catch (error) {
      if (error instanceof ZodError) {
        return NextResponse.json(
          { error: error.issues[0]?.message ?? 'Invalid contextual demo request.' },
          { status: 400 },
        );
      }
      return NextResponse.json({ error: 'Request body must be valid JSON.' }, { status: 400 });
    }

    const resolved = await resolveProfile(context, dependencies.getProfile);
    if (resolved === null) {
      return NextResponse.json({ error: 'Contextual demo not found.' }, { status: 404 });
    }

    try {
      const payload: HostedContextualDemoResponse = await dependencies.runWorkflow({
        ...clientRequest,
        profile: resolved.profile,
      });
      try {
        return NextResponse.json(sanitizeContextualDemoResponse(payload), {
          headers: { 'cache-control': 'no-store' },
        });
      } catch (error) {
        console.error(
          'contextual demo workflow contract failed',
          safeErrorForLog(error instanceof Error ? error : undefined),
        );
        return NextResponse.json(
          { error: 'The protected contextual workflow returned an invalid response.' },
          { status: 502 },
        );
      }
    } catch (error) {
      if (error instanceof ContextualDemoBudgetExceededError) {
        return NextResponse.json(
          { error: 'Contextual demo budget reached. Try again later.' },
          { status: 429 },
        );
      }
      console.error(
        'contextual demo request failed',
        safeErrorForLog(error instanceof Error ? error : undefined),
      );
      return NextResponse.json(
        { error: 'The protected contextual demo is temporarily unavailable.' },
        { status: 503 },
      );
    }
  }

  return { GET, POST };
}

async function resolveProfile(
  context: RouteContext,
  getProfile: ContextualDemoHandlersDependencies['getProfile'],
) {
  const { category } = await context.params;
  const profile = await getProfile(category);
  if (profile === null || !isContextualScenarioId(profile.scenario_id)) return null;
  return {
    scenarioId: profile.scenario_id,
    profile: {
      companyName: profile.company_name,
      userProfile: profile.user_profile,
      workflow: profile.workflow,
      riskBoundary: profile.risk_boundary,
      rule: profile.rule,
      approvalStep: profile.approval_step,
      recordShown: profile.record_shown,
      scenarioId: profile.scenario_id,
    },
  };
}

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
      const oldest = hits.keys().next().value as string | undefined;
      if (oldest === undefined) break;
      hits.delete(oldest);
    }
    hits.set(key, { count: 1, resetAt: now + RATE_LIMIT_WINDOW_MS });
    return false;
  }
  current.count += 1;
  return current.count > RATE_LIMIT_MAX;
}

function clientAddress(request: Request): string {
  return (
    request.headers.get('x-vercel-forwarded-for')?.split(',')[0]?.trim() ||
    request.headers.get('cf-connecting-ip') ||
    request.headers.get('x-real-ip') ||
    request.headers.get('x-forwarded-for')?.split(',')[0]?.trim() ||
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

function safeErrorForLog(error: Error | undefined): string {
  if (error === undefined) return 'unknown error';
  return `${error.name}: ${error.message}`.slice(0, 500);
}

const handlers = createContextualDemoHandlers();
export const GET = handlers.GET;
export const POST = handlers.POST;
