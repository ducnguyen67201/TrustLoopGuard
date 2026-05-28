import { createArenaAdapter, type ArenaAdapterServer } from '../arena/adapter';
import {
  chatBreakCases,
  mockProviderReplyFor,
  openAiDemoConfig,
  proxySupportAgent,
  realProviderSystemPrompt,
} from '../proxy/config';

const host = process.env.RAW_AGENT_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.RAW_AGENT_PORT ?? '8787', 10);

async function main(): Promise<void> {
  const adapter = await createArenaAdapter({
    host,
    port,
    profile: proxySupportAgent,
    async chat({ message }) {
      const content = await rawReplyFor(message);
      return {
        content,
        finishReason: 'stop',
        verdict: null,
        phase: null,
        traceId: null,
      };
    },
  });

  printReady(adapter);

  const shutdown = (): void => {
    void adapter.close().finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

async function rawReplyFor(userMessage: string): Promise<string> {
  if (openAiDemoConfig.apiKey) {
    return rawOpenAiReplyFor(userMessage);
  }

  const breakCase = chatBreakCases.find((candidate) => candidate.userMessage === userMessage);
  return breakCase ? mockProviderReplyFor(breakCase) : "I don't have a scripted answer for that prompt.";
}

async function rawOpenAiReplyFor(userMessage: string): Promise<string> {
  const response = await fetch(`${openAiDemoConfig.baseUrl}/v1/chat/completions`, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${openAiDemoConfig.apiKey}`,
      'content-type': 'application/json',
    },
    body: JSON.stringify({
      model: openAiDemoConfig.model,
      messages: [
        {
          role: 'system',
          content: realProviderSystemPrompt(),
        },
        { role: 'user', content: userMessage },
      ],
    }),
  });

  const bodyText = await response.text();
  const body = parseOpenAiChatResponse(bodyText);
  if (!response.ok) {
    throw new Error(body.error?.message ?? `OpenAI request failed with ${response.status}`);
  }

  const reply = body.choices?.[0]?.message?.content?.trim();
  if (!reply) throw new Error('OpenAI response did not include assistant content');
  return reply;
}

interface OpenAiChatResponse {
  choices?: Array<{ message?: { content?: string } }>;
  error?: { message?: string };
}

function parseOpenAiChatResponse(bodyText: string): OpenAiChatResponse {
  try {
    return JSON.parse(bodyText) as OpenAiChatResponse;
  } catch {
    return {};
  }
}

function printReady(adapter: ArenaAdapterServer): void {
  process.stdout.write('raw agent: ready\n');
  process.stdout.write(`agent   : ${proxySupportAgent.displayName}\n`);
  process.stdout.write(`listen  : ${adapter.url}\n`);
  process.stdout.write(`profile : ${adapter.url}/arena/profile\n`);
  process.stdout.write(`chat    : ${adapter.url}/arena/chat\n\n`);
  process.stdout.write(`provider: ${openAiDemoConfig.apiKey ? 'openai' : 'local mock'}\n\n`);
}

main().catch((error) => {
  process.stderr.write(`raw agent failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
