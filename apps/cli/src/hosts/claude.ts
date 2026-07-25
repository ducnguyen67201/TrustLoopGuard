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

const EVENTS = ['PreToolUse', 'PostToolUse', 'PostToolUseFailure'];
const MINIMUM_VERSION = '2.1.0';

export const claudeAdapter: HostAdapter = {
  id: 'claude',

  detect(context) {
    return detectHost('claude', MINIMUM_VERSION, context.env);
  },

  async install(context) {
    const detection = await this.detect(context);
    if (detection.compatibility === 'unsupported' && context.allowUnsupported !== true) {
      throw new Error(
        `Claude Code ${detection.version ?? 'unknown'} is unsupported; upgrade to ${MINIMUM_VERSION} or pass --allow-unsupported`,
      );
    }
    await mergeHookEvents(
      context.paths.claudeSettingsFile,
      EVENTS,
      commandHandler('node', {
        args: [context.paths.commandHookFile, '--host', 'claude'],
      }),
    );
  },

  async inspect(context) {
    const detection = await this.detect(context);
    const installed = await hasManagedHookEvents(context.paths.claudeSettingsFile, EVENTS);
    const remediation =
      compatibilityRemediation(detection, MINIMUM_VERSION) ??
      (installed
        ? 'Restart Claude Code and verify the handlers with /hooks'
        : 'Run install --target claude');
    return {
      ...baseStatus(
        'claude',
        detection,
        installed,
        context.runtimePresent,
        detection.compatibility,
      ),
      activation: installed ? 'configured' : 'inactive',
      coverage: installed ? 'universal' : 'none',
      exceptions: [],
      remediation,
    };
  },

  uninstall(context) {
    return removeHookEvents(context.paths.claudeSettingsFile, EVENTS);
  },
};
