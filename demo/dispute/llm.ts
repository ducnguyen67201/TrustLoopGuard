// Claude decision helper — raw fetch to the Anthropic Messages API, no SDK
// dependency (mirrors demo/agent-demo-adapter/llm.ts, which calls OpenAI the
// same way). Returns parsed JSON, or null when no key is set or the call/parse
// fails, so callers fall back to a deterministic decision and the demo still
// runs offline.
const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;
const ANTHROPIC_MODEL = process.env.ANTHROPIC_MODEL ?? 'claude-haiku-4-5-20251001';

interface AnthropicMessagesResponse {
  content?: Array<{ type?: string; text?: string }>;
}

export async function decideWithClaude({
  system,
  user,
}: {
  system: string;
  user: string;
}): Promise<unknown | null> {
  if (ANTHROPIC_API_KEY === undefined || ANTHROPIC_API_KEY.trim() === '') return null;

  try {
    const res = await fetch('https://api.anthropic.com/v1/messages', {
      method: 'POST',
      headers: {
        'x-api-key': ANTHROPIC_API_KEY,
        'anthropic-version': '2023-06-01',
        'content-type': 'application/json',
      },
      body: JSON.stringify({
        model: ANTHROPIC_MODEL,
        max_tokens: 512,
        system,
        messages: [{ role: 'user', content: user }],
      }),
    });
    if (!res.ok) return null;

    const body = (await res.json()) as AnthropicMessagesResponse;
    const text = body.content?.find((part) => part.type === 'text')?.text?.trim();
    if (text === undefined || text === '') return null;
    return JSON.parse(stripCodeFence(text));
  } catch {
    return null;
  }
}

function stripCodeFence(value: string): string {
  const trimmed = value.trim();
  const match = trimmed.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/i);
  return match?.[1]?.trim() ?? trimmed;
}
