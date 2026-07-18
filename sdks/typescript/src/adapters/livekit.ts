import type { ToolAdapterCandidate, ToolRuntimeValue } from '../tool-discovery.js';
import type { GuardAgentSessionRunOptions, GuardAgentRunWarning } from '../guard.js';

export interface LiveKitCloseEventLike {
  reason?: string;
  error?: object | null;
}

export type LiveKitCloseListener = (event: LiveKitCloseEventLike) => void | Promise<void>;

export interface LiveKitAgentSessionLike {
  on(event: 'close', listener: LiveKitCloseListener): object | void;
  off?(event: 'close', listener: LiveKitCloseListener): object | void;
}

export interface LiveKitRunOptions {
  externalId: GuardAgentSessionRunOptions['externalId'];
  kind?: GuardAgentSessionRunOptions['kind'];
  metadata?: GuardAgentSessionRunOptions['metadata'];
  onLifecycleWarning?: (warning: GuardAgentRunWarning) => void;
}

/**
 * Bind one automatic TrustLoopGuard Run to a LiveKit AgentSession lifecycle
 * without adding a runtime dependency on the LiveKit package.
 */
export function liveKitRun(
  session: LiveKitAgentSessionLike,
  opts: LiveKitRunOptions,
): GuardAgentSessionRunOptions {
  return {
    scope: 'session',
    externalId: opts.externalId,
    kind: opts.kind ?? 'live_call',
    ...(opts.metadata === undefined ? {} : { metadata: opts.metadata }),
    ...(opts.onLifecycleWarning === undefined
      ? {}
      : { onLifecycleWarning: opts.onLifecycleWarning }),
    registerEnd(finish) {
      const listener: LiveKitCloseListener = (event) => finish(liveKitRunStatus(event));
      session.on('close', listener);
      if (typeof session.off !== 'function') return;
      return () => {
        session.off?.('close', listener);
      };
    },
  };
}

function liveKitRunStatus(event: LiveKitCloseEventLike): 'completed' | 'failed' | 'canceled' {
  if (event.reason === 'error' || event.error != null) return 'failed';
  if (event.reason === 'job_shutdown') return 'canceled';
  return 'completed';
}

export function normalizeLiveKitTool(
  owner: object,
  registryKey: string,
  replaceOwner: (replacement: object) => boolean,
): ToolAdapterCandidate | undefined {
  const execute = Reflect.get(owner, 'execute', owner);
  const name = stringProperty(owner, 'name') ?? stringProperty(owner, 'id');
  if (typeof execute !== 'function' || name === undefined) return undefined;

  const candidate: ToolAdapterCandidate = {
    framework: 'livekit',
    registryKey,
    name,
    execute,
    owner,
    replaceOwner,
  };
  const description = stringProperty(owner, 'description');
  if (description !== undefined) candidate.description = description;
  const inputSchema = objectProperty(owner, 'parameters');
  if (inputSchema !== undefined) candidate.inputSchema = inputSchema;
  return candidate;
}

function stringProperty(target: object, property: PropertyKey): string | undefined {
  const value = Reflect.get(target, property, target) as ToolRuntimeValue;
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

function objectProperty(target: object, property: PropertyKey): object | undefined {
  const value = Reflect.get(target, property, target) as ToolRuntimeValue;
  return value !== null && typeof value === 'object' ? value : undefined;
}
