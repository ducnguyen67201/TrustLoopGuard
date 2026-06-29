import { NextResponse } from 'next/server';
import { z } from 'zod';

import { errorResponse } from '@/app/api/_shared';
import { isAllowedAgentTargetUrl } from '@/lib/redteam-core';
import { rustApiForAuthorizedWorkspace } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

const MAX_WORKFLOW_DEFINITION_BYTES = 1_000_000;

const stringListSchema = z
  .array(z.string().trim().min(1))
  .max(50)
  .transform((items) => Array.from(new Set(items)));

const workflowDefinitionSchema = z
  .object({
    source: z.string().trim().min(1),
    definition: z.record(z.string(), z.unknown()),
  })
  .refine(
    (value) =>
      Buffer.byteLength(JSON.stringify(value.definition), 'utf8') <= MAX_WORKFLOW_DEFINITION_BYTES,
    { message: 'workflow definition must be 1 MB or smaller' },
  );

const workflowRequirementSchema = z.object({
  name: z.string().trim().min(1, 'workflow requirement name is required'),
  requiredBefore: stringListSchema.default([]),
  sensitiveSteps: stringListSchema.default([]),
});

const updateAgentSchema = z
  .object({
    displayName: z.string().trim().min(1, 'displayName is required'),
    systemPrompt: z
      .string()
      .trim()
      .min(20, 'systemPrompt must be at least 20 characters')
      .optional(),
    workflowDefinition: workflowDefinitionSchema.optional(),
    workflowRequirements: z.array(workflowRequirementSchema).max(25).default([]),
    targetUrl: z
      .string()
      .trim()
      .refine(isAllowedAgentTargetUrl, 'targetUrl must be a loopback agent endpoint')
      .optional(),
    scope: z.object({
      inScope: stringListSchema,
      outOfScope: stringListSchema,
    }),
    authority: z.object({
      canPromise: stringListSchema,
      cannotPromise: stringListSchema,
    }),
    tone: z.object({
      target: z.string().trim().min(1, 'tone target is required'),
      forbidden: stringListSchema,
    }),
    escalationTriggers: stringListSchema,
  })
  .refine((value) => value.systemPrompt !== undefined || value.workflowDefinition !== undefined, {
    message: 'provide a systemPrompt or a workflowDefinition',
  });

interface RouteContext {
  params: Promise<{ id: string }>;
}

export async function GET(req: Request, context: RouteContext) {
  const { id } = await context.params;
  try {
    const agent = await rustApiForAuthorizedWorkspace(
      req,
      `/v1/agents/${encodeURIComponent(id)}`,
    );
    return NextResponse.json(agent);
  } catch (err) {
    return errorResponse(err);
  }
}

export async function PUT(req: Request, context: RouteContext) {
  const { id } = await context.params;
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'invalid JSON body' }, { status: 400 });
  }

  const parsed = updateAgentSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: parsed.error.issues[0]?.message ?? 'bad request' },
      { status: 400 },
    );
  }

  try {
    const agent = await rustApiForAuthorizedWorkspace(req, '/v1/agents', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        agent_id: id,
        display_name: parsed.data.displayName,
        ...(parsed.data.systemPrompt !== undefined
          ? { system_prompt: parsed.data.systemPrompt }
          : {}),
        ...(parsed.data.workflowDefinition !== undefined
          ? { workflow_definition: parsed.data.workflowDefinition }
          : {}),
        workflow_requirements: parsed.data.workflowRequirements.map((requirement) => ({
          name: requirement.name,
          required_before: requirement.requiredBefore,
          sensitive_steps: requirement.sensitiveSteps,
        })),
        ...(parsed.data.targetUrl !== undefined ? { target_url: parsed.data.targetUrl } : {}),
        scope: {
          in_scope: parsed.data.scope.inScope,
          out_of_scope: parsed.data.scope.outOfScope,
        },
        authority: {
          can_promise: parsed.data.authority.canPromise,
          cannot_promise: parsed.data.authority.cannotPromise,
        },
        tone: {
          target: parsed.data.tone.target,
          forbidden: parsed.data.tone.forbidden,
        },
        knowledge_sources: [],
        escalation_triggers: parsed.data.escalationTriggers,
      }),
    });
    return NextResponse.json(agent);
  } catch (err) {
    return errorResponse(err);
  }
}
