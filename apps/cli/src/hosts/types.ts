import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

import { atomicWriteJson, isJsonObject, readJsonObject } from '../managed-json.js';
import type {
  CliEnvironment,
  Compatibility,
  HostDetection,
  HostId,
  HostStatus,
  JsonObject,
  JsonValue,
} from '../types.js';
import type { TrustLoopPaths } from '../paths.js';

const execFileAsync = promisify(execFile);
const MANAGED_STATUS = 'TrustLoopGuard is authorizing this tool';

export interface HostContext {
  env: CliEnvironment;
  paths: TrustLoopPaths;
  platform: NodeJS.Platform;
  runtimePresent: boolean;
}

export interface HostInstallContext extends HostContext {
  allowUnsupported: boolean;
}

export interface HostAdapter {
  readonly id: HostId;
  detect(context: HostContext): Promise<HostDetection>;
  install(context: HostInstallContext): Promise<void>;
  inspect(context: HostContext): Promise<HostStatus>;
  uninstall(context: HostContext): Promise<void>;
}

interface CommandHookHandler extends JsonObject {
  type: 'command';
  command: string;
  args?: JsonValue[];
  commandWindows?: string;
  statusMessage: string;
  timeout: number;
}

export function commandHandler(
  command: string,
  options: { args?: string[]; commandWindows?: string },
): CommandHookHandler {
  return {
    type: 'command',
    command,
    ...(options.args === undefined ? {} : { args: options.args }),
    ...(options.commandWindows === undefined ? {} : { commandWindows: options.commandWindows }),
    statusMessage: MANAGED_STATUS,
    timeout: 330,
  };
}

export async function mergeHookEvents(
  file: string,
  events: string[],
  handler: CommandHookHandler,
): Promise<void> {
  const root = (await readJsonObject(file)) ?? {};
  const hooksValue = root['hooks'];
  const hooks = hooksValue === undefined ? {} : requireObject(hooksValue, `${file} hooks`);
  for (const event of events) {
    const value = hooks[event];
    const groups = value === undefined ? [] : requireArray(value, `${file} hooks.${event}`);
    const cleaned = removeManagedFromGroups(groups);
    hooks[event] = [...cleaned, { hooks: [handler] }];
  }
  root['hooks'] = hooks;
  await atomicWriteJson(file, root, { backup: true });
}

export async function removeHookEvents(file: string, events: string[]): Promise<void> {
  const root = await readJsonObject(file);
  if (root === undefined) return;
  const hooksValue = root['hooks'];
  if (hooksValue === undefined) return;
  const hooks = requireObject(hooksValue, `${file} hooks`);
  for (const event of events) {
    const value = hooks[event];
    if (value === undefined) continue;
    const cleaned = removeManagedFromGroups(requireArray(value, `${file} hooks.${event}`));
    if (cleaned.length === 0) delete hooks[event];
    else hooks[event] = cleaned;
  }
  root['hooks'] = hooks;
  await atomicWriteJson(file, root, { backup: true });
}

export async function hasManagedHookEvents(file: string, events: string[]): Promise<boolean> {
  const root = await readJsonObject(file);
  if (root === undefined) return false;
  const hooksValue = root['hooks'];
  if (hooksValue === undefined || !isJsonObject(hooksValue)) return false;
  return events.every((event) => {
    const groups = hooksValue[event];
    return Array.isArray(groups) && groups.some(groupHasManagedHandler);
  });
}

function removeManagedFromGroups(groups: JsonValue[]): JsonValue[] {
  return groups.flatMap((group) => {
    if (!isJsonObject(group)) return [group];
    const handlersValue = group['hooks'];
    if (!Array.isArray(handlersValue)) return [group];
    const handlers = handlersValue.filter((handler) => !isManagedHandler(handler));
    if (handlers.length === 0) return [];
    return [{ ...group, hooks: handlers }];
  });
}

function groupHasManagedHandler(group: JsonValue): boolean {
  if (!isJsonObject(group) || !Array.isArray(group['hooks'])) return false;
  return group['hooks'].some(isManagedHandler);
}

function isManagedHandler(value: JsonValue): boolean {
  return isJsonObject(value) && value['statusMessage'] === MANAGED_STATUS;
}

function requireObject(value: JsonValue, source: string): JsonObject {
  if (!isJsonObject(value)) throw new Error(`${source} must be a JSON object`);
  return value;
}

function requireArray(value: JsonValue, source: string): JsonValue[] {
  if (!Array.isArray(value)) throw new Error(`${source} must be a JSON array`);
  return value;
}

export async function detectHost(
  executable: string,
  minimumVersion: string,
  env: CliEnvironment,
): Promise<HostDetection> {
  try {
    const result = await execFileAsync(executable, ['--version'], {
      env: { ...process.env, ...env },
      timeout: 3_000,
    });
    const version = parseVersion(`${result.stdout} ${result.stderr}`);
    return {
      found: true,
      version,
      compatibility:
        version === null
          ? 'unknown'
          : compareVersions(version, minimumVersion) >= 0
            ? 'supported'
            : 'unsupported',
      executable,
    };
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      return { found: false, version: null, compatibility: 'unknown', executable };
    }
    return { found: true, version: null, compatibility: 'unknown', executable };
  }
}

export function parseVersion(value: string): string | null {
  return value.match(/\b(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)\b/)?.[1] ?? null;
}

export function compareVersions(left: string, right: string): number {
  const leftParts = left.split(/[.-]/).slice(0, 3).map(Number);
  const rightParts = right.split(/[.-]/).slice(0, 3).map(Number);
  for (let index = 0; index < 3; index += 1) {
    const difference = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);
    if (difference !== 0) return Math.sign(difference);
  }
  return 0;
}

export function compatibilityRemediation(
  detection: HostDetection,
  minimumVersion: string,
): string | null {
  if (!detection.found) return `Install ${detection.executable} or choose another --target`;
  if (detection.compatibility === 'unsupported') {
    return `Upgrade ${detection.executable} to ${minimumVersion} or newer`;
  }
  if (detection.compatibility === 'unknown') {
    return `Verify ${detection.executable} --version manually`;
  }
  return null;
}

export function baseStatus(
  id: HostId,
  detection: HostDetection,
  installed: boolean,
  runtimePresent: boolean,
  compatibility: Compatibility,
): Pick<HostStatus, 'id' | 'installed' | 'runtimePresent' | 'version' | 'compatibility'> {
  return {
    id,
    installed,
    runtimePresent,
    version: detection.version,
    compatibility,
  };
}
