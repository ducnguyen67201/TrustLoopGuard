import { readFile, rm } from 'node:fs/promises';
import { dirname, relative } from 'node:path';

import { atomicWriteText } from '../managed-json.js';
import { baseStatus, compatibilityRemediation, detectHost, type HostAdapter } from './types.js';

const MINIMUM_VERSION = '1.18.5';
const LOADER_MARKER = 'TrustLoopGuard managed OpenCode tool gate';

export function openCodeLoader(pluginFile: string, runtimeFile: string): string {
  let importPath = relative(dirname(pluginFile), runtimeFile).replaceAll('\\', '/');
  if (!importPath.startsWith('.')) importPath = `./${importPath}`;
  return `// ${LOADER_MARKER}\nexport { TrustLoopGuardPlugin } from ${JSON.stringify(importPath)};\n`;
}

export const openCodeAdapter: HostAdapter = {
  id: 'opencode',

  detect(context) {
    return detectHost('opencode', MINIMUM_VERSION, context.env);
  },

  async install(context) {
    const detection = await this.detect(context);
    if (detection.compatibility === 'unsupported' && context.allowUnsupported !== true) {
      throw new Error(
        `OpenCode ${detection.version ?? 'unknown'} is unsupported; upgrade to ${MINIMUM_VERSION} or pass --allow-unsupported`,
      );
    }
    const existing = await readLoader(context.paths.openCodePluginFile);
    if (existing !== undefined && !existing.startsWith(`// ${LOADER_MARKER}\n`)) {
      throw new Error(
        `${context.paths.openCodePluginFile} already exists and is not managed by TrustLoopGuard`,
      );
    }
    await atomicWriteText(
      context.paths.openCodePluginFile,
      openCodeLoader(context.paths.openCodePluginFile, context.paths.openCodeRuntimeFile),
      { backup: true, mode: 0o600 },
    );
  },

  async inspect(context) {
    const detection = await this.detect(context);
    const installed = await hasLoader(context.paths.openCodePluginFile);
    return {
      ...baseStatus(
        'opencode',
        detection,
        installed,
        context.runtimePresent,
        detection.compatibility,
      ),
      activation: installed ? 'configured' : 'inactive',
      coverage: installed ? 'universal' : 'none',
      exceptions: [],
      remediation:
        compatibilityRemediation(detection, MINIMUM_VERSION) ??
        (installed
          ? 'Restart OpenCode and run a harmless tool to verify the plugin'
          : 'Run install --target opencode'),
    };
  },

  async uninstall(context) {
    if (await hasLoader(context.paths.openCodePluginFile)) {
      await rm(context.paths.openCodePluginFile, { force: true });
    }
  },
};

async function hasLoader(file: string): Promise<boolean> {
  return (await readLoader(file))?.startsWith(`// ${LOADER_MARKER}\n`) ?? false;
}

async function readLoader(file: string): Promise<string | undefined> {
  try {
    return await readFile(file, 'utf8');
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
    throw error;
  }
}
