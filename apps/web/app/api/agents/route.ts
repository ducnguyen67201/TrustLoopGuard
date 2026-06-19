import { NextResponse } from 'next/server';
import { z } from 'zod';
import { tlClientForRequest } from '@/lib/server/tl-client';
import { errorResponse } from '@/app/api/_shared';
import { isAllowedAgentTargetUrl } from '@/lib/redteam-core';

export const runtime = 'nodejs';

// Raw imported agent definition (e.g. an n8n workflow export). The hardening
// loop's attack planner reads this to tailor attacks; kept verbatim. Capped so a
// huge graph can't be stored and re-analysed (BFS) on every plan call.
const MAX_WORKFLOW_DEFINITION_BYTES = 1_000_000;
const workflowDefinitionSchema = z
  .object({
    source: z.string().trim().min(1),
    definition: z.record(z.string(), z.unknown()),
  })
  .refine(
    (v) => Buffer.byteLength(JSON.stringify(v.definition), 'utf8') <= MAX_WORKFLOW_DEFINITION_BYTES,
    {
      message: 'workflow definition must be 1 MB or smaller',
    },
  );

// A chat agent needs a prompt; a workflow agent needs its definition. Require
// at least one so we never store an agent the planner can't reason about.
const createAgentSchema = z
  .object({
    displayName: z.string().trim().min(1, 'displayName is required'),
    systemPrompt: z
      .string()
      .trim()
      .min(20, 'systemPrompt must be at least 20 characters')
      .optional(),
    workflowDefinition: workflowDefinitionSchema.optional(),
    // Loopback-only: the dispatch SSRF guard is authoritative, but reject early
    // for a clear error at import time.
    targetUrl: z
      .string()
      .trim()
      .refine(isAllowedAgentTargetUrl, 'targetUrl must be a loopback agent endpoint')
      .optional(),
  })
  .refine((v) => v.systemPrompt !== undefined || v.workflowDefinition !== undefined, {
    message: 'provide a systemPrompt or a workflowDefinition',
  });

export async function GET(req: Request) {
  try {
    const result = await (await tlClientForRequest(req)).listAgents();
    return NextResponse.json(result);
  } catch (err) {
    return errorResponse(err);
  }
}

export async function POST(req: Request) {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'invalid JSON body' }, { status: 400 });
  }

  const parsed = createAgentSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: parsed.error.issues[0]?.message ?? 'bad request' },
      { status: 400 },
    );
  }

  const agentId = crypto.randomUUID();
  try {
    const agent = await (
      await tlClientForRequest(req)
    ).upsertAgent({
      agent_id: agentId,
      display_name: parsed.data.displayName,
      // Omit (don't set to undefined) absent optionals — the generated type
      // uses exactOptionalPropertyTypes.
      ...(parsed.data.systemPrompt !== undefined
        ? { system_prompt: parsed.data.systemPrompt }
        : {}),
      ...(parsed.data.workflowDefinition !== undefined
        ? { workflow_definition: parsed.data.workflowDefinition }
        : {}),
      ...(parsed.data.targetUrl !== undefined ? { target_url: parsed.data.targetUrl } : {}),
      scope: {
        in_scope: ['customer support and product questions'],
        out_of_scope: ['medical advice', 'legal advice', 'guaranteed refunds'],
      },
      authority: {
        can_promise: ['handoff to a teammate', 'share approved help-center information'],
        cannot_promise: ['refunds', 'medical outcomes', 'legal outcomes'],
      },
      tone: {
        target: 'clear-professional',
        forbidden: ['overconfident', 'dismissive'],
      },
      knowledge_sources: [],
      escalation_triggers: ['medical advice', 'legal advice', 'refund guarantee'],
    });
    return NextResponse.json(agent, { status: 201 });
  } catch (err) {
    return errorResponse(err);
  }
}
