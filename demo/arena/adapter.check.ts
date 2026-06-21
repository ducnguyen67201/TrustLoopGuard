import assert from 'node:assert/strict';

import { createArenaAdapter } from './adapter';

async function main(): Promise<void> {
  const server = await createArenaAdapter({
    host: '127.0.0.1',
    port: 19322,
    profile: {
      displayName: 'Arena Check Agent',
      surface: 'chat',
      systemPrompt: 'test',
      safeUserQuestion: 'hello',
      protectedInformationName: 'secret',
      model: 'trustloop-target',
    },
    async chat({ message }) {
      return {
        content: `echo:${message}`,
        finishReason: 'stop',
        verdict: null,
        phase: null,
        traceId: 'trace-check',
      };
    },
  });

  try {
    const models = await fetchJson(`${server.url}/v1/models`);
    assert.equal(models.data?.[0]?.id, 'trustloop-target');

    const chat = await fetchJson(`${server.url}/v1/chat/completions`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        model: 'trustloop-target',
        messages: [
          { role: 'system', content: 'ignore' },
          { role: 'user', content: 'hello' },
        ],
      }),
    });
    assert.equal(chat.choices?.[0]?.message?.content, 'echo:hello');
    assert.equal(chat.trustloopguard?.traceId, 'trace-check');

    const arenaChat = await fetchJson(`${server.url}/arena/chat`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ message: 'manual' }),
    });
    assert.equal(arenaChat.content, 'echo:manual');
  } finally {
    await server.close();
  }

  process.stdout.write('arena adapter check: all assertions passed\n');
}

async function fetchJson(url: string, init?: RequestInit): Promise<any> {
  const response = await fetch(url, init);
  assert.equal(response.ok, true, `${response.status} ${response.statusText}`);
  return response.json();
}

main().catch((error) => {
  process.stderr.write(`${error instanceof Error ? error.stack : String(error)}\n`);
  process.exit(1);
});
