import OpenAI from 'openai';
import { NextResponse } from 'next/server';
import { z } from 'zod';
import { env } from '@/env';
import {
  POLICY_ACTIONS,
  POLICY_MATCH_TYPES,
  POLICY_SEVERITIES,
  policyDraftSchema,
  type PolicyDraft,
} from '@/lib/policy-draft';

export const runtime = 'nodejs';

const requestSchema = z.object({
  prompt: z.string().trim().min(3, 'describe the policy in a few words'),
});

const SYSTEM_PROMPT =
  'You write TrustLoopGuard guardrail policies. Given a short natural-language description, return a policy draft as JSON. Prefer literal matches for specific phrases, regex for patterns. Default action is block; use rewrite only when a clear safe replacement exists; use escalate for ambiguous high-stakes cases. The id must be kebab-case (lowercase letters, digits, hyphens only).';

const POLICY_JSON_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['id', 'description', 'matchType', 'matchValue', 'action', 'severity', 'rewrite'],
  properties: {
    id: { type: 'string', description: 'kebab-case identifier' },
    description: { type: 'string' },
    matchType: { type: 'string', enum: [...POLICY_MATCH_TYPES] },
    matchValue: {
      type: 'string',
      description:
        'literal substring, or regex if matchType is regex. Pick the form most likely to catch the bad output.',
    },
    action: { type: 'string', enum: [...POLICY_ACTIONS] },
    severity: { type: 'string', enum: [...POLICY_SEVERITIES] },
    rewrite: {
      type: ['string', 'null'],
      description: 'safe replacement text. only when action is rewrite, otherwise null.',
    },
  },
} as const;

export async function POST(req: Request) {
  const apiKey = env.OPENAI_API_KEY;
  if (!apiKey) {
    return NextResponse.json(
      { error: 'OPENAI_API_KEY is not configured on the server.' },
      { status: 503 },
    );
  }

  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'invalid JSON body' }, { status: 400 });
  }

  const parsed = requestSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: parsed.error.issues[0]?.message ?? 'bad request' },
      { status: 400 },
    );
  }

  const client = new OpenAI({ apiKey });

  let completion;
  try {
    completion = await client.chat.completions.create({
      model: 'gpt-4o-mini',
      messages: [
        { role: 'system', content: SYSTEM_PROMPT },
        { role: 'user', content: parsed.data.prompt },
      ],
      response_format: {
        type: 'json_schema',
        json_schema: {
          name: 'policy_draft',
          strict: true,
          schema: POLICY_JSON_SCHEMA,
        },
      },
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'OpenAI request failed';
    return NextResponse.json({ error: message }, { status: 502 });
  }

  const raw = completion.choices[0]?.message.content;
  if (!raw) {
    return NextResponse.json({ error: 'model returned empty response' }, { status: 502 });
  }

  let modelOutput: unknown;
  try {
    modelOutput = JSON.parse(raw);
  } catch {
    return NextResponse.json({ error: 'model returned non-JSON output' }, { status: 502 });
  }

  // Coerce null rewrite to undefined for our draft schema (strict mode forces non-null union).
  if (
    modelOutput !== null &&
    typeof modelOutput === 'object' &&
    'rewrite' in modelOutput &&
    (modelOutput as { rewrite: unknown }).rewrite === null
  ) {
    delete (modelOutput as { rewrite?: unknown }).rewrite;
  }

  const draftParse = policyDraftSchema.safeParse(modelOutput);
  if (!draftParse.success) {
    return NextResponse.json(
      { error: 'model returned invalid policy shape', issues: draftParse.error.issues },
      { status: 502 },
    );
  }

  const draft: PolicyDraft = draftParse.data;
  return NextResponse.json({ draft });
}
