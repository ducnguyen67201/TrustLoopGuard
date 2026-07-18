import { normalizeLiveKitTool } from './adapters/livekit.js';
import { normalizeMastraTool } from './adapters/mastra.js';
import { normalizeOpenAiAgentsTool } from './adapters/openai-agents.js';
import type { AutomaticRunController, Client } from './client.js';
import type { AuthorizationDecision } from './generated/AuthorizationDecision.js';
import type { GuardEvent } from './generated/GuardEvent.js';
import type { SideEffectClass } from './generated/SideEffectClass.js';
import type { ToolIdentity } from './generated/ToolIdentity.js';
import type { UpsertToolMetadataRequest } from './generated/UpsertToolMetadataRequest.js';

export const ToolDiscoveryMode = {
  Auto: 'auto',
  Off: 'off',
} as const;
export type ToolDiscoveryMode = (typeof ToolDiscoveryMode)[keyof typeof ToolDiscoveryMode];

export const ToolRegistrationMode = {
  Off: 'off',
  BestEffort: 'best_effort',
  Strict: 'strict',
} as const;
export type ToolRegistrationMode = (typeof ToolRegistrationMode)[keyof typeof ToolRegistrationMode];

export const ToolDiscoveryWarningCode = {
  AgentToolsUnavailable: 'agent_tools_unavailable',
  ToolNotExecutable: 'tool_not_executable',
  ToolNotWritable: 'tool_not_writable',
  ToolResolverNotWritable: 'tool_resolver_not_writable',
  RegistrationFailed: 'registration_failed',
} as const;
export type ToolDiscoveryWarningCode =
  (typeof ToolDiscoveryWarningCode)[keyof typeof ToolDiscoveryWarningCode];

export type ToolFramework = 'mastra' | 'openai-agents' | 'livekit' | 'generic';
export type ToolRuntimeValue =
  | object
  | string
  | number
  | boolean
  | bigint
  | symbol
  | null
  | undefined;
export type ToolExecute = (
  ...args: ToolRuntimeValue[]
) => ToolRuntimeValue | PromiseLike<ToolRuntimeValue>;

export interface DiscoveredToolInfo {
  framework: ToolFramework;
  registryKey: string;
  name: string;
  description?: string;
  inputSchema?: object;
  outputSchema?: object;
}

export interface ToolAdapterCandidate extends DiscoveredToolInfo {
  execute: ToolExecute;
  owner: object;
  replaceOwner: (replacement: object) => boolean;
}

export interface GuardToolDiscoveryWarning {
  code: ToolDiscoveryWarningCode;
  message: string;
  framework?: ToolFramework;
  registryKey?: string;
  toolName?: string;
  cause?: Error;
}

export type GuardToolMetadataOverride = Partial<Omit<UpsertToolMetadataRequest, 'tool'>>;

export interface GuardToolDiscoveryOptions {
  /** Discover supported local tool registries. Defaults to `auto`. */
  discovery?: ToolDiscoveryMode;

  /**
   * Upsert discovered metadata lazily before the first tool call.
   * Defaults to `off` so decorating an agent does not create control-plane writes.
   */
  register?: ToolRegistrationMode;

  /** Optional warning hook for unsupported or partially visible tool surfaces. */
  onDiscoveryWarning?: (warning: GuardToolDiscoveryWarning) => void;

  /**
   * Provide authoritative metadata such as side-effect class, reversibility,
   * parameter roles, and approval requirements.
   */
  inferMetadata?: (tool: DiscoveredToolInfo) => GuardToolMetadataOverride;
}

export interface DecorateAgentToolsOptions {
  agentId: string;
  client: Client;
  context?: Exclude<GuardEvent['context'], null>;
  tools?: GuardToolDiscoveryOptions;
  automaticRun?: AutomaticRunController;
}

export class GuardedToolBlocked extends Error {
  readonly decision: AuthorizationDecision;
  readonly tool: DiscoveredToolInfo;

  constructor(tool: DiscoveredToolInfo, decision: AuthorizationDecision) {
    super(`TrustLoopGuard stopped tool "${tool.name}" with effect "${decision.effect}"`);
    this.name = 'GuardedToolBlocked';
    this.tool = tool;
    this.decision = decision;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

const WRAPPED_EXECUTES = new WeakSet<ToolExecute>();
const WRAPPED_OWNERS = new WeakMap<object, ToolExecute>();
const WRAPPED_RESOLVERS = new WeakSet<ToolExecute>();

export function decorateAgentTools(agent: object, opts: DecorateAgentToolsOptions): void {
  const discovery = opts.tools?.discovery ?? ToolDiscoveryMode.Auto;
  if (discovery === ToolDiscoveryMode.Off) return;

  let foundToolSurface = false;
  const directTools = Reflect.get(agent, 'tools', agent) as ToolRuntimeValue;
  if (Array.isArray(directTools)) {
    foundToolSurface = true;
    decorateArrayRegistry(directTools, opts);
  } else if (isRecord(directTools)) {
    foundToolSurface = true;
    decorateObjectRegistry(directTools, opts, 'generic');
  }

  const toolContext = Reflect.get(agent, 'toolCtx', agent) as ToolRuntimeValue;
  if (isRecord(toolContext)) {
    const liveKitTools = Reflect.get(toolContext, 'tools', toolContext) as ToolRuntimeValue;
    if (Array.isArray(liveKitTools)) {
      foundToolSurface = true;
      decorateLiveKitContext(toolContext, liveKitTools, opts);
    }
  }

  const resolver = Reflect.get(agent, 'getToolsForExecution', agent) as ToolRuntimeValue;
  if (isToolExecute(resolver)) {
    foundToolSurface = true;
    installMastraResolver(agent, resolver, opts);
  }

  if (!foundToolSurface) {
    warn(opts, {
      code: ToolDiscoveryWarningCode.AgentToolsUnavailable,
      message:
        'No supported local tool registry was exposed; final reply guarding remains available.',
    });
  }
}

export function toolSchemaHash(schema: object | undefined): string {
  const canonical = canonicalJson(schema ?? {});
  let hash = 0xcbf29ce484222325n;
  for (let index = 0; index < canonical.length; index += 1) {
    hash ^= BigInt(canonical.charCodeAt(index));
    hash = BigInt.asUintN(64, hash * 0x100000001b3n);
  }
  return `tlg-schema:fnv1a64:${hash.toString(16).padStart(16, '0')}`;
}

function decorateObjectRegistry(
  registry: Record<string, ToolRuntimeValue>,
  opts: DecorateAgentToolsOptions,
  framework: ToolFramework,
): void {
  for (const [registryKey, value] of Object.entries(registry)) {
    if (!isRecord(value)) {
      warnNotExecutable(opts, framework, registryKey);
      continue;
    }
    const candidate = normalizeMastraTool(
      value,
      registryKey,
      (replacement) => Reflect.set(registry, registryKey, replacement, registry),
      framework === 'generic' && looksLikeMastraTool(value) ? 'mastra' : framework,
    );
    if (candidate === undefined) {
      warnNotExecutable(opts, framework, registryKey);
      continue;
    }
    decorateCandidate(candidate, opts);
  }
}

function decorateArrayRegistry(
  registry: ToolRuntimeValue[],
  opts: DecorateAgentToolsOptions,
): void {
  for (let index = 0; index < registry.length; index += 1) {
    const value = registry[index];
    const registryKey = String(index);
    if (!isRecord(value)) {
      warnNotExecutable(opts, 'generic', registryKey);
      continue;
    }
    const replaceOwner = (replacement: object) =>
      Reflect.set(registry, index, replacement, registry);
    const candidate = looksLikeLiveKitTool(value)
      ? normalizeLiveKitTool(value, registryKey, replaceOwner)
      : normalizeOpenAiAgentsTool(value, registryKey, replaceOwner);
    if (candidate === undefined) {
      warnNotExecutable(
        opts,
        looksLikeLiveKitTool(value) ? 'livekit' : 'openai-agents',
        registryKey,
      );
      continue;
    }
    decorateCandidate(candidate, opts);
  }
}

function decorateLiveKitContext(
  toolContext: Record<string, ToolRuntimeValue>,
  registry: ToolRuntimeValue[],
  opts: DecorateAgentToolsOptions,
): void {
  const nextTools = [...registry];
  const updateTools = Reflect.get(toolContext, 'updateTools', toolContext) as ToolRuntimeValue;

  for (let index = 0; index < registry.length; index += 1) {
    const value = registry[index];
    const registryKey = String(index);
    if (!isRecord(value)) {
      warnNotExecutable(opts, 'livekit', registryKey);
      continue;
    }
    const replaceOwner = (replacement: object): boolean => {
      if (typeof updateTools !== 'function') return false;
      nextTools[index] = replacement;
      try {
        Reflect.apply(updateTools, toolContext, [nextTools]);
        return true;
      } catch {
        return false;
      }
    };
    const candidate = normalizeLiveKitTool(value, registryKey, replaceOwner);
    if (candidate === undefined) {
      warnNotExecutable(opts, 'livekit', registryKey);
      continue;
    }
    decorateCandidate(candidate, opts);
  }
}

function installMastraResolver(
  agent: object,
  resolver: ToolExecute,
  opts: DecorateAgentToolsOptions,
): void {
  if (WRAPPED_RESOLVERS.has(resolver)) return;

  const wrappedResolver: ToolExecute = async (...args) => {
    const resolved = await Reflect.apply(resolver, agent, args);
    if (isRecord(resolved)) {
      decorateObjectRegistry(resolved, opts, 'mastra');
    }
    return resolved as ToolRuntimeValue;
  };
  WRAPPED_RESOLVERS.add(wrappedResolver);

  if (!Reflect.set(agent, 'getToolsForExecution', wrappedResolver, agent)) {
    warn(opts, {
      code: ToolDiscoveryWarningCode.ToolResolverNotWritable,
      message: 'Mastra getToolsForExecution() could not be wrapped.',
      framework: 'mastra',
    });
  }
}

function decorateCandidate(tool: ToolAdapterCandidate, opts: DecorateAgentToolsOptions): void {
  if (WRAPPED_EXECUTES.has(tool.execute)) return;

  const existingWrapper = WRAPPED_OWNERS.get(tool.owner);
  if (existingWrapper !== undefined) {
    installExecute(tool, existingWrapper, opts);
    return;
  }

  const info = publicToolInfo(tool);
  const metadataOverride = opts.tools?.inferMetadata?.(info);
  const registrationMode = opts.tools?.register ?? ToolRegistrationMode.Off;
  const registrationRequest =
    registrationMode === ToolRegistrationMode.Off
      ? undefined
      : metadataRequest(info, metadataOverride);
  let registration: Promise<void> | undefined;

  const ensureRegistered = async (): Promise<void> => {
    if (registrationRequest === undefined) return;
    registration ??= opts.client
      .upsertToolMetadata(registrationRequest)
      .then(() => undefined)
      .catch((error) => {
        if (registrationMode === ToolRegistrationMode.Strict) throw error;
        warn(opts, {
          code: ToolDiscoveryWarningCode.RegistrationFailed,
          message: `Tool metadata registration failed for "${info.name}"; authorization will continue.`,
          framework: info.framework,
          registryKey: info.registryKey,
          toolName: info.name,
          cause: error instanceof Error ? error : new Error(String(error)),
        });
      });
    await registration;
  };

  const wrapped: ToolExecute = async (...args) => {
    const invoke = async (): Promise<ToolRuntimeValue> => {
      await ensureRegistered();
      const parameters = parametersFromArgs(info.framework, args);
      const toolIdentity: ToolIdentity = {
        server_id: info.framework,
        tool_name: info.name,
        schema_hash: toolSchemaHash(info.inputSchema),
      };
      const sideEffect = metadataOverride?.side_effect;
      const result = await opts.client.withAuthorizedAction<ToolRuntimeValue>(
        {
          agentId: opts.agentId,
          operation: info.name,
          parameters,
          toolIdentity,
          context: opts.context ?? null,
          ...(sideEffect !== undefined ? { sideEffect } : {}),
        },
        async (approved) => {
          const approvedArgs = argsWithApprovedParameters(
            info.framework,
            args,
            approved as Readonly<Record<string, ToolRuntimeValue>>,
          );
          return await Reflect.apply(tool.execute, tool.owner, approvedArgs);
        },
      );
      if (!result.executed) {
        throw new GuardedToolBlocked(info, result.decision);
      }
      return result.value;
    };

    return opts.automaticRun === undefined ? invoke() : opts.automaticRun.run(invoke);
  };

  if (installExecute(tool, wrapped, opts)) {
    WRAPPED_EXECUTES.add(wrapped);
    WRAPPED_OWNERS.set(tool.owner, wrapped);
  }
}

function installExecute(
  tool: ToolAdapterCandidate,
  wrapped: ToolExecute,
  opts: DecorateAgentToolsOptions,
): boolean {
  if (
    Reflect.set(tool.owner, 'execute', wrapped, tool.owner) &&
    Reflect.get(tool.owner, 'execute', tool.owner) === wrapped
  ) {
    return true;
  }

  const replacement = Object.assign(Object.create(Object.getPrototypeOf(tool.owner)), tool.owner);
  if (Reflect.set(replacement, 'execute', wrapped, replacement) && tool.replaceOwner(replacement)) {
    return true;
  }

  warn(opts, {
    code: ToolDiscoveryWarningCode.ToolNotWritable,
    message: `Tool "${tool.name}" was discovered but execute() could not be replaced.`,
    framework: tool.framework,
    registryKey: tool.registryKey,
    toolName: tool.name,
  });
  return false;
}

function parametersFromArgs(
  framework: ToolFramework,
  args: ToolRuntimeValue[],
): Record<string, ToolRuntimeValue> {
  if (args.length === 0) return {};
  const first = args[0];
  if (!isRecord(first)) return { value: first };

  if (framework === 'mastra') {
    const context = Reflect.get(first, 'context', first) as ToolRuntimeValue;
    if (isRecord(context)) return { ...context };
  }
  return { ...first };
}

function argsWithApprovedParameters(
  framework: ToolFramework,
  args: ToolRuntimeValue[],
  approved: Readonly<Record<string, ToolRuntimeValue>>,
): ToolRuntimeValue[] {
  if (args.length === 0) return args;
  const first = args[0];
  if (isRecord(first)) {
    if (framework === 'mastra') {
      const context = Reflect.get(first, 'context', first) as ToolRuntimeValue;
      if (isRecord(context)) {
        return [{ ...first, context: approved }, ...args.slice(1)];
      }
    }
    return [approved, ...args.slice(1)];
  }
  return [Reflect.has(approved, 'value') ? approved['value'] : first, ...args.slice(1)];
}

function metadataRequest(
  tool: DiscoveredToolInfo,
  override: GuardToolMetadataOverride | undefined,
): UpsertToolMetadataRequest {
  return {
    ...override,
    tool: tool.name,
    side_effect: override?.side_effect ?? ('api_mutation' satisfies SideEffectClass),
    reversible: override?.reversible ?? false,
    params: override?.params ?? [],
    enabled: override?.enabled ?? true,
  };
}

function publicToolInfo(tool: ToolAdapterCandidate): DiscoveredToolInfo {
  const info: DiscoveredToolInfo = {
    framework: tool.framework,
    registryKey: tool.registryKey,
    name: tool.name,
  };
  if (tool.description !== undefined) info.description = tool.description;
  if (tool.inputSchema !== undefined) info.inputSchema = tool.inputSchema;
  if (tool.outputSchema !== undefined) info.outputSchema = tool.outputSchema;
  return info;
}

function canonicalJson(value: ToolRuntimeValue): string {
  const seen = new WeakSet<object>();
  const normalize = (current: ToolRuntimeValue): ToolRuntimeValue => {
    if (
      current === null ||
      typeof current === 'string' ||
      typeof current === 'number' ||
      typeof current === 'boolean'
    ) {
      return current;
    }
    if (typeof current === 'bigint') return current.toString();
    if (typeof current === 'undefined' || typeof current === 'symbol') return null;
    if (typeof current === 'function') return `[function:${current.name || 'anonymous'}]`;
    if (seen.has(current)) return '[circular]';
    seen.add(current);
    if (Array.isArray(current)) return current.map((item) => normalize(item));

    const normalized: Record<string, ToolRuntimeValue> = {};
    for (const key of Object.keys(current).sort()) {
      const nested = Reflect.get(current, key, current) as ToolRuntimeValue;
      if (
        typeof nested === 'function' ||
        typeof nested === 'undefined' ||
        typeof nested === 'symbol'
      ) {
        continue;
      }
      normalized[key] = normalize(nested);
    }
    return normalized;
  };
  return JSON.stringify(normalize(value)) ?? 'null';
}

function isRecord(value: ToolRuntimeValue): value is Record<string, ToolRuntimeValue> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isToolExecute(value: ToolRuntimeValue): value is ToolExecute {
  return typeof value === 'function';
}

function looksLikeMastraTool(tool: object): boolean {
  return (
    typeof Reflect.get(tool, 'id', tool) === 'string' ||
    Reflect.has(tool, 'inputSchema') ||
    Reflect.has(tool, 'outputSchema')
  );
}

function looksLikeLiveKitTool(tool: object): boolean {
  return (
    typeof Reflect.get(tool, 'id', tool) === 'string' ||
    typeof Reflect.get(tool, 'flags', tool) === 'number'
  );
}

function warnNotExecutable(
  opts: DecorateAgentToolsOptions,
  framework: ToolFramework,
  registryKey: string,
): void {
  warn(opts, {
    code: ToolDiscoveryWarningCode.ToolNotExecutable,
    message: `Tool entry "${registryKey}" has no local execute() function and cannot be guarded before execution.`,
    framework,
    registryKey,
  });
}

function warn(opts: DecorateAgentToolsOptions, warning: GuardToolDiscoveryWarning): void {
  opts.tools?.onDiscoveryWarning?.(warning);
}
