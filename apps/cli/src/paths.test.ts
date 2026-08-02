import { mkdir, mkdtemp, realpath, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, describe, expect, test } from 'vitest';

import { canonicalizeProject, ensureOwnedDirectory, resolveFeatherlaneAIPaths } from './paths.js';

const directories: string[] = [];

afterEach(async () => {
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true })));
});

describe('paths', () => {
  test('resolves XDG, host, and CODEX_HOME locations', () => {
    const paths = resolveFeatherlaneAIPaths(
      {
        XDG_CONFIG_HOME: '/config',
        CODEX_HOME: '/custom-codex',
      },
      'linux',
      '/home/user',
    );
    expect(paths.registryFile).toBe('/config/featherlane-ai/registry.json');
    expect(paths.claudeSettingsFile).toBe('/home/user/.claude/settings.json');
    expect(paths.codexHooksFile).toBe('/custom-codex/hooks.json');
    expect(paths.openCodePluginFile).toBe('/config/opencode/plugins/featherlane-ai.mjs');
  });

  test('resolves Windows AppData without changing environment variables', () => {
    const env = { APPDATA: 'C:\\Users\\A\\AppData\\Roaming', CODEX_HOME: 'C:\\Codex' };
    const paths = resolveFeatherlaneAIPaths(env, 'win32', 'C:\\Users\\A');
    expect(paths.configRoot).toContain('featherlane-ai');
    expect(env.CODEX_HOME).toBe('C:\\Codex');
  });

  test('canonicalizes projects and rejects symlinked owned directories', async () => {
    const parent = await mkdtemp(join(tmpdir(), 'featherlane-ai-paths-'));
    directories.push(parent);
    const project = join(parent, 'project');
    await mkdir(project);
    expect(await canonicalizeProject(project)).toBe(await realpath(project));
    const target = join(parent, 'target');
    const link = join(parent, 'link');
    await mkdir(target);
    await symlink(target, link);
    await expect(ensureOwnedDirectory(link)).rejects.toThrow(/real directory/);
  });
});
