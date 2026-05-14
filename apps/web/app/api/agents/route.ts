import { NextResponse } from 'next/server';
import { z } from 'zod';
import { tlClient } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

const createAgentSchema = z.object({
  displayName: z.string().trim().min(1, 'displayName is required'),
  systemPrompt: z.string().trim().min(20, 'systemPrompt must be at least 20 characters'),
});

export async function GET() {
  try {
    const result = await tlClient().listAgents();
    return NextResponse.json(result);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
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
    const agent = await tlClient().upsertAgent({
      agent_id: agentId,
      display_name: parsed.data.displayName,
      system_prompt: parsed.data.systemPrompt,
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
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
