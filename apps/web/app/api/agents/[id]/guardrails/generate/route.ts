import { NextResponse } from 'next/server';
import { tlClient } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

// Server -> tl-server proxy for POST /v1/agents/{id}/guardrails/generate.
// The actual LLM call lives on tl-server (it holds the provider key).
// Forwards the typed response straight back to the browser.
export async function POST(_req: Request, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  try {
    const result = await tlClient().generateGuardrails(id);
    return NextResponse.json(result);
  } catch (err) {
    // The SDK throws typed errors for 4xx/5xx; we don't try to translate
    // every variant here — the dialog surfaces the message verbatim.
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
