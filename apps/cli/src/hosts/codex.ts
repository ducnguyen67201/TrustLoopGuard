import {
  baseStatus,
  commandHandler,
  compatibilityRemediation,
  detectHost,
  hasManagedHookEvents,
  mergeHookEvents,
  removeHookEvents,
  type HostAdapter,
} from './types.js';

const EVENTS = ['PreToolUse', 'PostToolUse', 'Stop', 'SessionEnd'];
const MINIMUM_VERSION = '0.124.0';
const COVERAGE_EXCEPTIONS = [
  'read_file and grep are not proven to emit PreToolUse in the tested Codex contract',
  'new built-in handlers remain ungated until Codex exposes a PreToolUse payload',
  'user-level PostToolUse has had version-specific discovery regressions',
];

export function quotePosix(value: string): string {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

export function quoteWindows(value: string): string {
  return `"${value.replaceAll('"', '\\"')}"`;
}

export const codexAdapter: HostAdapter = {
  id: 'codex',

  detect(context) {
    return detectHost('codex', MINIMUM_VERSION, context.env);
  },

  async install(context) {
    const detection = await this.detect(context);
    if (detection.compatibility === 'unsupported' && context.allowUnsupported !== true) {
      throw new Error(
        `Codex ${detection.version ?? 'unknown'} is unsupported; upgrade to ${MINIMUM_VERSION} or pass --allow-unsupported`,
      );
    }
    const posix = `node ${quotePosix(context.paths.commandHookFile)} --host codex`;
    const windows = `node ${quoteWindows(context.paths.commandHookFile)} --host codex`;
    await mergeHookEvents(
      context.paths.codexHooksFile,
      EVENTS,
      commandHandler(posix, { commandWindows: windows }),
    );
  },

  async inspect(context) {
    const detection = await this.detect(context);
    const installed = await hasManagedHookEvents(context.paths.codexHooksFile, EVENTS);
    const compatibility = detection.compatibility;
    return {
      ...baseStatus('codex', detection, installed, context.runtimePresent, compatibility),
      activation: installed ? 'trust_required' : 'inactive',
      coverage: installed ? 'host_emitted_only' : 'none',
      exceptions: installed ? COVERAGE_EXCEPTIONS : [],
      remediation:
        compatibilityRemediation(detection, MINIMUM_VERSION) ??
        (installed
          ? 'Restart Codex, open /hooks, review the Featherlane AI commands, and approve trust'
          : 'Run install --target codex'),
    };
  },

  uninstall(context) {
    return removeHookEvents(context.paths.codexHooksFile, EVENTS);
  },
};
