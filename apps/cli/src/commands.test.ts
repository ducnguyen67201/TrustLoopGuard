import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';

import { afterEach, describe, expect, test, vi } from 'vitest';

import { runCommand, runtimeIsIntact } from './commands.js';
import { resolveTrustLoopPaths } from './paths.js';
import { CliError, type CommandContext } from './types.js';

const RUNTIME_FILES = [
  'bridge.js',
  'command-hook.js',
  'opencode-plugin.js',
  'runtime-types.js',
  'state.js',
  'wire.js',
];
const directories: string[] = [];

async function setup(version = '2.1.133'): Promise<{
  root: string;
  project: string;
  context: CommandContext;
  output: string[];
}> {
  const root = await mkdtemp(join(tmpdir(), 'tlg-commands-'));
  directories.push(root);
  const home = join(root, 'home');
  const config = join(root, 'config');
  const project = join(root, 'project');
  const bin = join(root, 'bin');
  const runtime = join(root, 'runtime-source');
  await Promise.all([mkdir(home), mkdir(config), mkdir(project), mkdir(bin), mkdir(runtime)]);
  const claude = join(bin, 'claude');
  await writeFile(claude, `#!/bin/sh\necho ${version} Claude Code\n`);
  await chmod(claude, 0o700);
  await Promise.all(RUNTIME_FILES.map((file) => writeFile(join(runtime, file), `// ${file}\n`)));
  const output: string[] = [];
  const context: CommandContext = {
    cwd: project,
    homeDirectory: home,
    platform: process.platform,
    runtimeSourceDirectory: runtime,
    env: {
      HOME: home,
      XDG_CONFIG_HOME: config,
      PATH: `${bin}${delimiter}${process.env.PATH ?? ''}`,
      TLG_API_KEY: 'tl_live_command_test_only',
    },
    stdout: (message) => output.push(message),
    stderr: (message) => output.push(`stderr:${message}`),
  };
  return { root, project, context, output };
}

afterEach(async () => {
  vi.unstubAllGlobals();
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true })));
});

describe('commands', () => {
  test('installs, reports, diagnoses, and uninstalls a Claude gate', async () => {
    const { project, context, output } = await setup();
    expect(
      await runCommand(
        {
          command: 'install',
          project,
          json: true,
          agentId: 'coding-agent',
          allowUnsupported: false,
          target: ['claude'],
          url: 'https://api.example.test',
        },
        context,
      ),
    ).toBe(0);
    const paths = resolveTrustLoopPaths(context.env, process.platform, context.homeDirectory);
    expect(await runtimeIsIntact(paths)).toBe(true);
    const settings = await readFile(paths.claudeSettingsFile, 'utf8');
    expect(settings).toContain('PreToolUse');
    expect(settings).not.toContain('tl_live_command_test_only');
    expect(await readFile(paths.registryFile, 'utf8')).not.toContain('tl_live_command_test_only');

    output.length = 0;
    expect(await runCommand({ command: 'status', project, json: true }, context)).toBe(0);
    expect(output.join('')).toContain('"registered": true');

    vi.stubGlobal(
      'fetch',
      vi.fn<typeof fetch>().mockResolvedValue(new Response('ok', { status: 200 })),
    );
    output.length = 0;
    expect(await runCommand({ command: 'doctor', project, json: true }, context)).toBe(0);
    expect(output.join('')).toContain('"reachable": true');

    output.length = 0;
    expect(
      await runCommand(
        { command: 'uninstall', project, json: true, all: true, target: 'all' },
        context,
      ),
    ).toBe(0);
    await expect(readFile(paths.registryFile, 'utf8')).rejects.toThrow();
    expect(await runtimeIsIntact(paths)).toBe(false);
  });

  test('shares adapters across projects and removes them after the last project', async () => {
    const { root, project, context } = await setup();
    const second = join(root, 'second-project');
    await mkdir(second);
    const install = (targetProject: string) =>
      runCommand(
        {
          command: 'install',
          project: targetProject,
          json: false,
          agentId: 'agent',
          allowUnsupported: false,
          target: ['claude'],
          url: 'https://api.example.test',
        },
        context,
      );
    await install(project);
    await install(second);
    const paths = resolveTrustLoopPaths(context.env, process.platform, context.homeDirectory);
    await runCommand(
      { command: 'uninstall', project, json: false, all: true, target: 'all' },
      context,
    );
    expect(await readFile(paths.claudeSettingsFile, 'utf8')).toContain('TrustLoopGuard');
    await runCommand(
      { command: 'uninstall', project: second, json: false, all: true, target: 'all' },
      context,
    );
    expect(await readFile(paths.claudeSettingsFile, 'utf8')).not.toContain('TrustLoopGuard');
  });

  test('fails before writes for a missing key, invalid URL, and unsupported host', async () => {
    const missing = await setup();
    delete missing.context.env.TLG_API_KEY;
    await expect(
      runCommand(
        {
          command: 'install',
          project: missing.project,
          json: false,
          agentId: 'agent',
          allowUnsupported: false,
          target: ['claude'],
          url: 'https://api.example.test',
        },
        missing.context,
      ),
    ).rejects.toThrow(/TLG_API_KEY/);

    const invalid = await setup();
    const invalidUrl = new URL('https://example.test');
    invalidUrl.username = 'placeholder-user';
    await expect(
      runCommand(
        {
          command: 'install',
          project: invalid.project,
          json: false,
          agentId: 'agent',
          allowUnsupported: false,
          target: ['claude'],
          url: invalidUrl.href,
        },
        invalid.context,
      ),
    ).rejects.toBeInstanceOf(CliError);

    const unsupported = await setup('1.9.0');
    await expect(
      runCommand(
        {
          command: 'install',
          project: unsupported.project,
          json: false,
          agentId: 'agent',
          allowUnsupported: false,
          target: ['claude'],
          url: 'https://api.example.test',
        },
        unsupported.context,
      ),
    ).rejects.toMatchObject({ exitCode: 3 });
  });

  test('rolls back runtime and registry when host configuration is malformed', async () => {
    const { project, context } = await setup();
    const paths = resolveTrustLoopPaths(context.env, process.platform, context.homeDirectory);
    await mkdir(join(context.homeDirectory!, '.claude'));
    await writeFile(paths.claudeSettingsFile, '{');
    await expect(
      runCommand(
        {
          command: 'install',
          project,
          json: false,
          agentId: 'agent',
          allowUnsupported: false,
          target: ['claude'],
          url: 'https://api.example.test',
        },
        context,
      ),
    ).rejects.toThrow(/malformed JSON/);
    await expect(readFile(paths.registryFile, 'utf8')).rejects.toThrow();
    expect(await readFile(paths.claudeSettingsFile, 'utf8')).toBe('{');
    expect(await runtimeIsIntact(paths)).toBe(false);
  });
});
