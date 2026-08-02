import { mkdir, mkdtemp, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, describe, expect, test, vi } from 'vitest';

import { runCommandHook } from './command-hook.js';

const directories: string[] = [];

async function config(): Promise<{ root: string; project: string }> {
  const root = await mkdtemp(join(tmpdir(), 'featherlane-ai-command-hook-'));
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
          targets: ['claude'],
          cliVersion: '1',
          runtimeVersion: '1',
          createdAt: 'now',
          updatedAt: 'now',
        },
      ],
    }),
  );
  return { root, project };
}

afterEach(async () => {
  vi.unstubAllGlobals();
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true })));
});

describe('command hook protocol', () => {
  test('returns structured deny output with protocol-clean stderr', async () => {
    const { root, project } = await config();
    vi.stubGlobal(
      'fetch',
      vi
        .fn<typeof fetch>()
        .mockResolvedValue(
          new Response(JSON.stringify({ effect: 'deny', reason: 'blocked', trace_id: 'trace-1' })),
        ),
    );
    const result = await runCommandHook(
      JSON.stringify({
        hook_event_name: 'PreToolUse',
        tool_name: 'mcp__server__tool',
        tool_use_id: 'call-1',
        session_id: 'session-1',
        cwd: project,
        tool_input: { value: true },
      }),
      ['--host', 'claude'],
      project,
      root,
      { FEATHERLANE_AI_API_KEY: 'tl_live_test_only' },
    );
    expect(JSON.parse(result.stdout)).toMatchObject({
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        permissionDecision: 'deny',
      },
    });
    expect(result.stderr).toBe('');
  });

  test('denies malformed input only in managed projects', async () => {
    const { root, project } = await config();
    const managed = await runCommandHook('{', ['--host', 'claude'], project, root, {});
    expect(JSON.parse(managed.stdout)).toMatchObject({
      hookSpecificOutput: { permissionDecision: 'deny' },
    });
    const unmanaged = await runCommandHook(
      '{',
      ['--host', 'claude'],
      join(root, 'elsewhere'),
      root,
      {},
    );
    expect(unmanaged).toEqual({ stdout: '', stderr: '' });
  });

  test('fails closed for invalid managed events and reconciles post events quietly', async () => {
    const { root, project } = await config();
    const invalid = JSON.stringify({
      hook_event_name: 'UnknownHook',
      cwd: project,
    });
    expect(
      JSON.parse((await runCommandHook(invalid, ['--host', 'claude'], project, root, {})).stdout),
    ).toMatchObject({
      hookSpecificOutput: { permissionDecision: 'deny' },
    });
    expect(
      await runCommandHook(invalid, ['--host', 'claude'], join(root, 'elsewhere'), root, {}),
    ).toEqual({ stdout: '', stderr: '' });

    const post = (hookEventName: string) =>
      JSON.stringify({
        hook_event_name: hookEventName,
        tool_name: 'Bash',
        tool_use_id: 'missing-call',
        session_id: 'missing-session',
        cwd: project,
      });
    expect(
      await runCommandHook(post('PostToolUse'), ['--host', 'codex'], project, root, {}),
    ).toEqual({ stdout: '', stderr: '' });
    expect(
      await runCommandHook(post('SessionEnd'), ['--host', 'claude'], project, root, {}),
    ).toEqual({ stdout: '', stderr: '' });
  });
});
