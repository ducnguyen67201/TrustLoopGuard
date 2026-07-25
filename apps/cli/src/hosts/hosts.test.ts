import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, describe, expect, test } from 'vitest';

import { resolveTrustLoopPaths } from '../paths.js';
import type { HostInstallContext } from './types.js';
import {
  commandHandler,
  compatibilityRemediation,
  compareVersions,
  detectHost,
  hasManagedHookEvents,
  mergeHookEvents,
  parseVersion,
  removeHookEvents,
} from './types.js';
import { codexAdapter, quotePosix, quoteWindows } from './codex.js';
import { openCodeAdapter, openCodeLoader } from './opencode.js';

const directories: string[] = [];

async function fixture(): Promise<{ root: string; context: HostInstallContext }> {
  const root = await mkdtemp(join(tmpdir(), 'tlg-hosts-'));
  directories.push(root);
  const bin = join(root, 'bin');
  await mkdir(bin);
  const codex = join(bin, 'codex');
  await writeFile(codex, '#!/bin/sh\necho codex-cli 0.144.6\n');
  await chmod(codex, 0o700);
  const openCode = join(bin, 'opencode');
  await writeFile(openCode, '#!/bin/sh\necho opencode 1.18.5\n');
  await chmod(openCode, 0o700);
  const env = {
    HOME: join(root, 'home'),
    XDG_CONFIG_HOME: join(root, 'config'),
    CODEX_HOME: join(root, 'codex-home'),
    PATH: bin,
  };
  return {
    root,
    context: {
      env,
      paths: resolveTrustLoopPaths(env, process.platform, env.HOME),
      platform: process.platform,
      runtimePresent: true,
      allowUnsupported: false,
    },
  };
}

afterEach(async () => {
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true })));
});

describe('host adapters', () => {
  test('parses and compares host versions', () => {
    expect(parseVersion('codex-cli 0.144.6')).toBe('0.144.6');
    expect(parseVersion('no version')).toBeNull();
    expect(compareVersions('0.144.6', '0.124.0')).toBe(1);
    expect(compareVersions('0.123.9', '0.124.0')).toBe(-1);
  });

  test('merges managed handlers idempotently and preserves foreign hooks', async () => {
    const { root } = await fixture();
    const file = join(root, 'settings.json');
    await writeFile(
      file,
      JSON.stringify({
        foreign: true,
        hooks: {
          PreToolUse: [{ matcher: 'Bash', hooks: [{ type: 'command', command: 'other' }] }],
        },
      }),
    );
    const handler = commandHandler('node', { args: ['/runtime.js'] });
    await mergeHookEvents(file, ['PreToolUse'], handler);
    await mergeHookEvents(file, ['PreToolUse'], handler);
    expect(await hasManagedHookEvents(file, ['PreToolUse'])).toBe(true);
    const settings = JSON.parse(await readFile(file, 'utf8')) as {
      foreign: boolean;
      hooks: { PreToolUse: Array<{ matcher?: string; hooks: Array<{ statusMessage?: string }> }> };
    };
    expect(settings.foreign).toBe(true);
    expect(settings.hooks.PreToolUse).toHaveLength(2);
    await removeHookEvents(file, ['PreToolUse']);
    expect(
      (JSON.parse(await readFile(file, 'utf8')) as typeof settings).hooks.PreToolUse,
    ).toHaveLength(1);
  });

  test('rejects malformed hook containers and reports host compatibility', async () => {
    const { root, context } = await fixture();
    const file = join(root, 'settings.json');
    const handler = commandHandler('node', { args: ['/runtime.js'] });
    await writeFile(file, JSON.stringify({ hooks: 'invalid' }));
    await expect(mergeHookEvents(file, ['PreToolUse'], handler)).rejects.toThrow(/object/);
    await writeFile(file, JSON.stringify({ hooks: { PreToolUse: 'invalid' } }));
    await expect(mergeHookEvents(file, ['PreToolUse'], handler)).rejects.toThrow(/array/);
    expect(await hasManagedHookEvents(join(root, 'absent.json'), ['PreToolUse'])).toBe(false);

    const missing = await detectHost('missing-host', '1.0.0', context.env);
    expect(missing.found).toBe(false);
    expect(compatibilityRemediation(missing, '1.0.0')).toContain('Install');

    const unsupported = await detectHost('codex', '9.0.0', context.env);
    expect(unsupported.compatibility).toBe('unsupported');
    expect(compatibilityRemediation(unsupported, '9.0.0')).toContain('Upgrade');

    const unknownExecutable = join(root, 'bin', 'unknown-host');
    await writeFile(unknownExecutable, '#!/bin/sh\necho development-build\n');
    await chmod(unknownExecutable, 0o700);
    const unknown = await detectHost('unknown-host', '1.0.0', context.env);
    expect(unknown.compatibility).toBe('unknown');
    expect(compatibilityRemediation(unknown, '1.0.0')).toContain('Verify');
    expect(
      compatibilityRemediation(
        { found: true, version: '1.0.0', compatibility: 'supported', executable: 'host' },
        '1.0.0',
      ),
    ).toBeNull();
  });

  test('renders exact Codex hook schema and truthful coverage', async () => {
    const { context } = await fixture();
    await codexAdapter.install(context);
    const hooks = JSON.parse(await readFile(context.paths.codexHooksFile, 'utf8')) as {
      hooks: Record<string, Array<{ hooks: Array<Record<string, string | number>> }>>;
    };
    const handler = hooks.hooks['PreToolUse']?.[0]?.hooks[0];
    expect(handler).toMatchObject({
      type: 'command',
      statusMessage: 'TrustLoopGuard is authorizing this tool',
      timeout: 330,
    });
    expect(handler?.['command']).toContain('--host codex');
    expect(handler?.['commandWindows']).toContain('--host codex');
    expect(hooks.hooks['PreToolUse']?.[0]).not.toHaveProperty('matcher');

    const status = await codexAdapter.inspect(context);
    expect(status.coverage).toBe('host_emitted_only');
    expect(status.activation).toBe('trust_required');
    expect(status.exceptions.join(' ')).toContain('read_file');
  });

  test('quotes host command paths and renders a relative OpenCode loader', () => {
    expect(quotePosix("/tmp/it's/hook.js")).toBe("'/tmp/it'\\''s/hook.js'");
    expect(quoteWindows('C:\\A \"B\"\\hook.js')).toBe('"C:\\A \\"B\\"\\hook.js"');
    expect(
      openCodeLoader(
        '/config/opencode/plugins/trustloopguard.mjs',
        '/config/trustloopguard/runtime/opencode-plugin.js',
      ),
    ).toContain('../../trustloopguard/runtime/opencode-plugin.js');
  });

  test('installs and removes only the managed OpenCode loader', async () => {
    const { context } = await fixture();
    await openCodeAdapter.install(context);
    expect(await readFile(context.paths.openCodePluginFile, 'utf8')).toContain(
      'TrustLoopGuard managed OpenCode tool gate',
    );
    expect(await openCodeAdapter.inspect(context)).toMatchObject({
      installed: true,
      activation: 'configured',
      coverage: 'universal',
    });
    await openCodeAdapter.uninstall(context);
    await expect(readFile(context.paths.openCodePluginFile, 'utf8')).rejects.toThrow();
    expect(await openCodeAdapter.inspect(context)).toMatchObject({
      installed: false,
      activation: 'inactive',
    });

    await writeFile(context.paths.openCodePluginFile, 'export const foreign = true;\n');
    await expect(openCodeAdapter.install(context)).rejects.toThrow(/not managed/);
    await openCodeAdapter.uninstall(context);
    expect(await readFile(context.paths.openCodePluginFile, 'utf8')).toContain('foreign');
  });
});
