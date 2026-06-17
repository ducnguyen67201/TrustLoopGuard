import { OPENAI_API_KEY, OPENAI_MODEL } from '../shared/env';

interface OpenAiChatResponse {
  choices?: Array<{
    message?: {
      content?: string | null;
    };
  }>;
}

export async function draftWithLlm({
  system,
  user,
  temperature = 0.2,
  model = OPENAI_MODEL,
}: {
  system: string;
  user: string;
  temperature?: number;
  model?: string;
}): Promise<string | null> {
  if (OPENAI_API_KEY === undefined || OPENAI_API_KEY.trim() === '') return null;

  try {
    const res = await fetch('https://api.openai.com/v1/chat/completions', {
      method: 'POST',
      headers: {
        authorization: `Bearer ${OPENAI_API_KEY}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify({
        model,
        temperature,
        messages: [
          { role: 'system', content: system },
          { role: 'user', content: user },
        ],
      }),
    });
    if (!res.ok) return null;

    const body = (await res.json()) as OpenAiChatResponse;
    return body.choices?.[0]?.message?.content?.trim() || null;
  } catch {
    return null;
  }
}

export async function draftJsonWithLlm({
  system,
  user,
  model,
}: {
  system: string;
  user: string;
  model?: string;
}): Promise<unknown | null> {
  const text = await draftWithLlm({ system, user, temperature: 0, model });
  if (text === null) return null;

  try {
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
