import { chmod, lstat, mkdir, realpath } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

import type { CliEnvironment } from './types.js';

export interface TrustLoopPaths {
  configRoot: string;
  registryFile: string;
  lockFile: string;
  runtimeDirectory: string;
  runtimeManifestFile: string;
  commandHookFile: string;
  openCodeRuntimeFile: string;
  stateDirectory: string;
  claudeSettingsFile: string;
  codexHooksFile: string;
  openCodePluginFile: string;
}

export function resolveTrustLoopPaths(
  env: CliEnvironment = process.env,
  platform: NodeJS.Platform = process.platform,
  homeDirectory: string = homedir(),
): TrustLoopPaths {
  const platformConfigRoot =
    platform === 'win32'
      ? env.APPDATA?.trim() || join(homeDirectory, 'AppData', 'Roaming')
      : env.XDG_CONFIG_HOME?.trim() || join(homeDirectory, '.config');
  const configRoot = join(platformConfigRoot, 'trustloopguard');
  const runtimeDirectory = join(configRoot, 'runtime');
  const codexHome = env.CODEX_HOME?.trim() || join(homeDirectory, '.codex');
  const openCodeRoot = join(platformConfigRoot, 'opencode');

  return {
    configRoot,
    registryFile: join(configRoot, 'registry.json'),
    lockFile: join(configRoot, '.install.lock'),
    runtimeDirectory,
    runtimeManifestFile: join(runtimeDirectory, 'manifest.json'),
    commandHookFile: join(runtimeDirectory, 'command-hook.js'),
    openCodeRuntimeFile: join(runtimeDirectory, 'opencode-plugin.js'),
    stateDirectory: join(configRoot, 'state'),
    claudeSettingsFile: join(homeDirectory, '.claude', 'settings.json'),
    codexHooksFile: join(codexHome, 'hooks.json'),
    openCodePluginFile: join(openCodeRoot, 'plugins', 'trustloopguard.mjs'),
  };
}

export async function canonicalizeProject(project: string): Promise<string> {
  return realpath(project);
}

export async function ensureOwnedDirectory(directory: string): Promise<void> {
  await mkdir(directory, { recursive: true, mode: 0o700 });
  const stats = await lstat(directory);
  if (!stats.isDirectory() || stats.isSymbolicLink()) {
    throw new Error(`${directory} must be a real directory`);
  }
  if (typeof process.getuid === 'function' && stats.uid !== process.getuid()) {
    throw new Error(`${directory} is owned by another user`);
  }
  if (process.platform !== 'win32') await chmod(directory, 0o700);
}
