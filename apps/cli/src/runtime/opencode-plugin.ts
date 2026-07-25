import type { Plugin } from '@opencode-ai/plugin';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { JsonObject, RuntimeEnvironment } from './runtime-types.js';
import {
  authorizeToolCall,
  cancelSessionLeases,
  completeToolCall,
  type BridgeOptions,
} from './bridge.js';
import type { HostToolCall } from './wire.js';

const installedConfigRoot = dirname(dirname(fileURLToPath(import.meta.url)));

export function createTrustLoopGuardPlugin(
  configRoot: string,
  environment: RuntimeEnvironment,
): Plugin {
  return async ({ directory, worktree }) => {
    const options: BridgeOptions = { configRoot, env: environment };
    const activeCalls = new Map<string, HostToolCall>();
    const projectRoot = worktree || directory;

    return {
      'tool.execute.before': async (input, output) => {
        const call = openCodeCall(
          'pre',
          input.tool,
          input.sessionID,
          input.callID,
          directory,
          projectRoot,
          output.args as JsonObject,
        );
        const result = await authorizeToolCall(call, options);
        if (!result.managed) return;
        if (!result.allowed) throw new Error(result.reason);
        activeCalls.set(callKey(call.sessionId, call.callId), call);
      },
      'tool.execute.after': async (input) => {
        const key = callKey(input.sessionID, input.callID);
        const call =
          activeCalls.get(key) ??
          openCodeCall(
            'post-success',
            input.tool,
            input.sessionID,
            input.callID,
            directory,
            projectRoot,
            input.args as JsonObject,
          );
        activeCalls.delete(key);
        await completeToolCall({ ...call, event: 'post-success' }, 'consumed', options);
      },
      event: async ({ event }) => {
        if (event.type === 'message.part.updated') {
          const part = event.properties.part;
          if (
            part.type === 'tool' &&
            part.state.status === 'error' &&
            typeof part.sessionID === 'string' &&
            typeof part.callID === 'string'
          ) {
            const key = callKey(part.sessionID, part.callID);
            const call = activeCalls.get(key);
            if (call !== undefined) {
              activeCalls.delete(key);
              await completeToolCall({ ...call, event: 'post-failure' }, 'canceled', options);
            }
          }
        }
        if (event.type === 'session.deleted') {
          const sessionId = event.properties.info.id;
          const call = openCodeCall(
            'session-end',
            'session',
            sessionId,
            '',
            directory,
            projectRoot,
            {},
          );
          await cancelSessionLeases(call, options);
        }
      },
      dispose: async () => {
        const sessions = new Set([...activeCalls.values()].map((call) => call.sessionId));
        activeCalls.clear();
        await Promise.all(
          [...sessions].map(async (sessionId) => {
            try {
              await cancelSessionLeases(
                openCodeCall('session-end', 'session', sessionId, '', directory, projectRoot, {}),
                options,
              );
            } catch {
              // The server's bounded lease expiry is the final reconciliation boundary.
            }
          }),
        );
      },
    };
  };
}

export const TrustLoopGuardPlugin: Plugin = createTrustLoopGuardPlugin(
  installedConfigRoot,
  process.env,
);

function openCodeCall(
  event: HostToolCall['event'],
  toolName: string,
  sessionId: string,
  callId: string,
  cwd: string,
  projectRoot: string,
  input: JsonObject,
): HostToolCall {
  return {
    host: 'opencode',
    event,
    toolName,
    callId,
    sessionId,
    cwd,
    projectRoot,
    input,
    hostVersion: '',
  };
}

function callKey(sessionId: string, callId: string): string {
  return `${sessionId}\0${callId}`;
}
