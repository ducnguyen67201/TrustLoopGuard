import { mkdir, mkdtemp, readdir, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import type { PluginInput } from '@opencode-ai/plugin';
import { afterEach, describe, expect, test, vi } from 'vitest';

import { createFeatherlaneAIPlugin } from './opencode-plugin.js';

const directories: string[] = [];

async function setup(): Promise<{ root: string; project: string; input: PluginInput }> {
  const root = await mkdtemp(join(tmpdir(), 'featherlane-ai-opencode-plugin-'));
  directories.push(root);
  const projectDirectory = join(root, 'project');
  await mkdir(projectDirectory);
  const project = await realpath(projectDirectory);
  await writeFile(
    join(root, 'registry.json'),
    JSON.stringify({
      version: 1,
      projects: [
        {
          root: project,
          url: 'https://api.example.test',
          agentId: 'agent',
          targets: ['opencode'],
          cliVersion: '1',
          runtimeVersion: '1',
          createdAt: 'now',
          updatedAt: 'now',
        },
      ],
    }),
  );
  const input = {
    directory: project,
    worktree: project,
  } as PluginInput;
  return { root, project, input };
}

afterEach(async () => {
  vi.unstubAllGlobals();
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true })));
});

describe('OpenCode runtime plugin', () => {
  test('throws before execution when Featherlane AI denies', async () => {
    const { root, input } = await setup();
    vi.stubGlobal(
      'fetch',
      vi
        .fn<typeof fetch>()
        .mockResolvedValue(
          new Response(JSON.stringify({ effect: 'deny', reason: 'blocked', trace_id: 'trace-1' })),
        ),
    );
    const hooks = await createFeatherlaneAIPlugin(root, {
      FEATHERLANE_AI_API_KEY: 'tl_live_test_only',
    })(input);
    await expect(
      hooks['tool.execute.before']?.(
        { tool: 'bash', sessionID: 'session-1', callID: 'call-1' },
        { args: { command: 'echo blocked' } },
      ),
    ).rejects.toThrow(/blocked/);
  });

  test('consumes after success and cancels outstanding leases on dispose', async () => {
    const { root, input } = await setup();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            effect: 'permit',
            reason: 'allowed',
            trace_id: 'trace-2',
            lease: { id: 'lease-1' },
          }),
        ),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({ status: 'consumed' })))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            effect: 'permit',
            reason: 'allowed',
            trace_id: 'trace-3',
            lease: { id: 'lease-2' },
          }),
        ),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({ status: 'canceled' })));
    vi.stubGlobal('fetch', fetchMock);
    const hooks = await createFeatherlaneAIPlugin(root, {
      FEATHERLANE_AI_API_KEY: 'tl_live_test_only',
    })(input);

    await hooks['tool.execute.before']?.(
      { tool: 'bash', sessionID: 'session-1', callID: 'call-1' },
      { args: { command: 'echo allowed' } },
    );
    await hooks['tool.execute.after']?.(
      {
        tool: 'bash',
        sessionID: 'session-1',
        callID: 'call-1',
        args: { command: 'echo allowed' },
      },
      { title: 'bash', output: 'allowed', metadata: {} },
    );
    expect(await readdir(join(root, 'state'))).toHaveLength(0);

    await hooks['tool.execute.before']?.(
      { tool: 'bash', sessionID: 'session-2', callID: 'call-2' },
      { args: { command: 'echo later' } },
    );
    await hooks.dispose?.();
    expect(await readdir(join(root, 'state'))).toHaveLength(0);
    const completionBodies = fetchMock.mock.calls
      .filter(([url]) => String(url).endsWith('/complete'))
      .map(
        ([, init]) =>
          JSON.parse(String(init?.body)) as {
            status: string;
            outcome: { hook_event_name: string };
          },
      );
    expect(completionBodies).toEqual([
      { status: 'consumed', outcome: { hook_event_name: 'post-success' } },
      { status: 'canceled', outcome: { hook_event_name: 'session-end' } },
    ]);
  });
});
