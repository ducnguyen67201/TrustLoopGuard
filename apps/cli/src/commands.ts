import { createHash } from 'node:crypto';
import { readFile, rm } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { claudeAdapter } from './hosts/claude.js';
import { codexAdapter } from './hosts/codex.js';
import { openCodeAdapter } from './hosts/opencode.js';
import type { HostAdapter, HostContext } from './hosts/types.js';
import {
  atomicWriteJson,
  atomicWriteText,
  isJsonObject,
  readJsonValue,
  rejectSymlink,
  withFileLock,
} from './managed-json.js';
import {
  canonicalizeProject,
  ensureOwnedDirectory,
  resolveFeatherlaneAIPaths,
  type FeatherlaneAIPaths,
} from './paths.js';
import {
  findRegistration,
  readRegistry,
  registeredTargets,
  removeRegistrationTargets,
  RUNTIME_VERSION,
  upsertRegistration,
  writeRegistry,
} from './registry.js';
import {
  HOST_IDS,
  CliError,
  type CliCommandOptions,
  type CommandContext,
  type HostId,
  type HostStatus,
  type JsonObject,
  type TargetSelection,
} from './types.js';
import { CLI_VERSION } from './version.js';

const DEFAULT_FEATHERLANE_AI_URL = 'https://api.featherlane.ai';
const RUNTIME_FILES = [
  'bridge.js',
  'command-hook.js',
  'opencode-plugin.js',
  'runtime-types.js',
  'state.js',
  'wire.js',
] as const;
const ADAPTERS: Record<HostId, HostAdapter> = {
  claude: claudeAdapter,
  codex: codexAdapter,
  opencode: openCodeAdapter,
};

interface RuntimeManifest extends JsonObject {
  runtimeVersion: string;
  cliVersion: string;
  files: JsonObject;
}

interface FileSnapshot {
  file: string;
  contents: string | undefined;
}

export async function runCommand(
  options: CliCommandOptions,
  context: CommandContext,
): Promise<0 | 3> {
  if (options.command === 'help') return 0;
  if (options.command === 'install') return installCommand(options, context);
  if (options.command === 'status') return statusCommand(options, context, false);
  if (options.command === 'doctor') return statusCommand(options, context, true);
  return uninstallCommand(options, context);
}

async function installCommand(
  options: Extract<CliCommandOptions, { command: 'install' }>,
  context: CommandContext,
): Promise<0 | 3> {
  if (!context.env.FEATHERLANE_AI_API_KEY?.trim()) {
    throw new CliError(
      'FEATHERLANE_AI_API_KEY is required in the environment; it is never accepted as a CLI argument',
      1,
    );
  }
  const project = await canonicalizeProject(options.project);
  const agentId = validateAgentId(options.agentId ?? context.env.FEATHERLANE_AI_AGENT_ID);
  const url = validateUrl(options.url ?? context.env.FEATHERLANE_AI_URL ?? DEFAULT_FEATHERLANE_AI_URL);
  const paths = commandPaths(context);
  await ensureOwnedDirectory(paths.configRoot);
  const adapterContext = await hostContext(paths, context);
  const targets = await selectTargets(options.target, adapterContext);
  const detections = await Promise.all(
    targets.map((target) => ADAPTERS[target].detect(adapterContext)),
  );
  for (const [index, detection] of detections.entries()) {
    const target = targets[index];
    if (target === undefined) continue;
    if (!detection.found) throw new CliError(`${target} is not installed or not available on PATH`);
    if (detection.compatibility === 'unsupported' && !options.allowUnsupported) {
      throw new CliError(
        `${target} ${detection.version ?? 'unknown'} is below the tested compatibility floor`,
        3,
        'Upgrade the host or rerun with --allow-unsupported after reviewing its hook contract.',
      );
    }
  }
  await Promise.all(targets.map((target) => ADAPTERS[target].inspect(adapterContext)));

  await withFileLock(paths.lockFile, async () => {
    const registry = await readRegistry(paths.registryFile);
    const existing = findRegistration(registry, project);
    const combinedTargets = uniqueTargets([...(existing?.targets ?? []), ...targets]);
    const nextRegistry = upsertRegistration(
      registry,
      {
        root: project,
        url,
        agentId,
        targets: combinedTargets,
        cliVersion: CLI_VERSION,
        runtimeVersion: RUNTIME_VERSION,
      },
      new Date().toISOString(),
    );
    const snapshots = await snapshotFiles(filesTouchedByInstall(paths, targets));
    try {
      await copyRuntime(paths, context.runtimeSourceDirectory);
      await writeRegistry(paths.registryFile, nextRegistry);
      const installContext = {
        ...adapterContext,
        runtimePresent: true,
        allowUnsupported: options.allowUnsupported,
      };
      for (const target of targets) await ADAPTERS[target].install(installContext);
    } catch (error) {
      await restoreSnapshots(snapshots);
      throw error;
    }
  });

  const statuses = await inspectHosts(targets, paths, context);
  printResult(
    options.json,
    {
      project,
      url,
      agentId,
      targets,
      statuses,
      next: restartInstructions(statuses),
    },
    context,
  );
  return statuses.some(isDegraded) ? 3 : 0;
}

async function statusCommand(
  options: Extract<CliCommandOptions, { command: 'doctor' | 'status' }>,
  context: CommandContext,
  doctor: boolean,
): Promise<0 | 3> {
  const paths = commandPaths(context);
  const project = await canonicalizeProject(options.project);
  const registry = await readRegistry(paths.registryFile);
  const registration = findRegistration(registry, project);
  const targets = registration?.targets ?? [];
  const runtimePresent = await runtimeIsIntact(paths);
  const statuses = await inspectHosts(targets, paths, context, runtimePresent);
  const health =
    doctor && registration !== undefined
      ? await checkHealth(registration.url)
      : { checked: false, reachable: false, note: 'not checked' };
  const keyPresent = Boolean(context.env.FEATHERLANE_AI_API_KEY?.trim());
  const result = {
    project,
    registered: registration !== undefined,
    registration: registration ?? null,
    runtimePresent,
    keyPresent,
    health,
    statuses,
    note:
      doctor && health.reachable
        ? 'Health proves server reachability only; runtime-key validity is confirmed by a real guarded event.'
        : null,
  };
  printResult(options.json, result, context);
  const degraded =
    registration === undefined ||
    !runtimePresent ||
    (doctor && (!keyPresent || !health.reachable)) ||
    statuses.some(isDegraded);
  return degraded ? 3 : 0;
}

async function uninstallCommand(
  options: Extract<CliCommandOptions, { command: 'uninstall' }>,
  context: CommandContext,
): Promise<0> {
  const paths = commandPaths(context);
  await ensureOwnedDirectory(paths.configRoot);
  const project = await canonicalizeProject(options.project);
  await withFileLock(paths.lockFile, async () => {
    const registry = await readRegistry(paths.registryFile);
    const registration = findRegistration(registry, project);
    if (registration === undefined) throw new CliError(`${project} is not registered`);
    const targets = options.all
      ? registration.targets
      : uninstallTargets(options.target, registration.targets);
    const nextRegistry = removeRegistrationTargets(
      registry,
      registration.root,
      targets.length === registration.targets.length ? 'all' : targets,
      new Date().toISOString(),
    );
    const remaining = registeredTargets(nextRegistry);
    const removable = targets.filter((target) => !remaining.has(target));
    const snapshots = await snapshotFiles([
      paths.registryFile,
      ...removable.map((target) => hostConfigFile(paths, target)),
      ...(removable.includes('opencode') ? [paths.openCodePluginFile] : []),
    ]);
    try {
      const adapterContext = await hostContext(paths, context);
      for (const target of removable) await ADAPTERS[target].uninstall(adapterContext);
      if (nextRegistry.projects.length === 0) {
        await rm(paths.registryFile, { force: true });
        await rm(paths.runtimeDirectory, { recursive: true, force: true });
      } else {
        await writeRegistry(paths.registryFile, nextRegistry);
      }
    } catch (error) {
      await restoreSnapshots(snapshots);
      throw error;
    }
    printResult(
      options.json,
      {
        project: registration.root,
        removedTargets: targets,
        retainedTargets: [...remaining],
        remainingProjects: nextRegistry.projects.map((entry) => entry.root),
      },
      context,
    );
  });
  return 0;
}

async function selectTargets(selection: TargetSelection, context: HostContext): Promise<HostId[]> {
  if (Array.isArray(selection)) return selection;
  if (selection === 'all') return [...HOST_IDS];
  const detections = await Promise.all(HOST_IDS.map((host) => ADAPTERS[host].detect(context)));
  const targets = HOST_IDS.filter((_, index) => detections[index]?.found === true);
  if (targets.length === 0) {
    throw new CliError('no supported coding-agent host was detected on PATH', 1);
  }
  return targets;
}

function uninstallTargets(selection: TargetSelection, registered: HostId[]): HostId[] {
  if (selection === 'all' || selection === 'auto') return [...registered];
  const missing = selection.filter((target) => !registered.includes(target));
  if (missing.length > 0)
    throw new CliError(`project is not registered for: ${missing.join(', ')}`, 2);
  return selection;
}

async function hostContext(
  paths: FeatherlaneAIPaths,
  context: CommandContext,
  runtimePresent = false,
): Promise<HostContext> {
  return {
    paths,
    env: context.env,
    platform: context.platform ?? process.platform,
    runtimePresent,
  };
}

async function inspectHosts(
  targets: HostId[],
  paths: FeatherlaneAIPaths,
  context: CommandContext,
  knownRuntimePresent?: boolean,
): Promise<HostStatus[]> {
  const runtimePresent = knownRuntimePresent ?? (await runtimeIsIntact(paths));
  const adapterContext = await hostContext(paths, context, runtimePresent);
  return Promise.all(
    targets.map(async (target): Promise<HostStatus> => {
      try {
        return await ADAPTERS[target].inspect(adapterContext);
      } catch (error) {
        const message = error instanceof Error ? error.message : 'host configuration is unreadable';
        return {
          id: target,
          installed: false,
          runtimePresent,
          version: null,
          compatibility: 'unknown',
          activation: 'unknown',
          coverage: 'none',
          exceptions: [message],
          remediation: `Repair the ${target} configuration, then rerun install`,
        };
      }
    }),
  );
}

async function copyRuntime(paths: FeatherlaneAIPaths, runtimeSourceDirectory?: string): Promise<void> {
  await ensureOwnedDirectory(paths.runtimeDirectory);
  await ensureOwnedDirectory(paths.stateDirectory);
  const sourceDirectory =
    runtimeSourceDirectory ?? join(dirname(fileURLToPath(import.meta.url)), 'runtime');
  const hashes: JsonObject = {};
  for (const file of RUNTIME_FILES) {
    const contents = await readFile(join(sourceDirectory, file), 'utf8');
    await atomicWriteText(join(paths.runtimeDirectory, file), contents, { mode: 0o600 });
    hashes[file] = sha256(contents);
  }
  await atomicWriteJson(join(paths.runtimeDirectory, 'package.json'), { type: 'module' });
  const manifest: RuntimeManifest = {
    runtimeVersion: RUNTIME_VERSION,
    cliVersion: CLI_VERSION,
    files: hashes,
  };
  await atomicWriteJson(paths.runtimeManifestFile, manifest);
}

export async function runtimeIsIntact(paths: FeatherlaneAIPaths): Promise<boolean> {
  try {
    const value = await readJsonValue(paths.runtimeManifestFile);
    if (
      value === undefined ||
      !isJsonObject(value) ||
      value['runtimeVersion'] !== RUNTIME_VERSION ||
      !isJsonObject(value['files'])
    ) {
      return false;
    }
    for (const file of RUNTIME_FILES) {
      const expected = value['files'][file];
      if (typeof expected !== 'string') return false;
      if (sha256(await readFile(join(paths.runtimeDirectory, file), 'utf8')) !== expected)
        return false;
    }
    return true;
  } catch {
    return false;
  }
}

function filesTouchedByInstall(paths: FeatherlaneAIPaths, targets: HostId[]): string[] {
  return [
    paths.registryFile,
    paths.runtimeManifestFile,
    join(paths.runtimeDirectory, 'package.json'),
    ...RUNTIME_FILES.map((file) => join(paths.runtimeDirectory, file)),
    ...targets.map((target) => hostConfigFile(paths, target)),
  ];
}

function hostConfigFile(paths: FeatherlaneAIPaths, target: HostId): string {
  if (target === 'claude') return paths.claudeSettingsFile;
  if (target === 'codex') return paths.codexHooksFile;
  return paths.openCodePluginFile;
}

function commandPaths(context: CommandContext): FeatherlaneAIPaths {
  return resolveFeatherlaneAIPaths(
    context.env,
    context.platform ?? process.platform,
    context.homeDirectory,
  );
}

async function snapshotFiles(files: string[]): Promise<FileSnapshot[]> {
  const snapshots: FileSnapshot[] = [];
  for (const file of [...new Set(files)]) {
    await rejectSymlink(file);
    try {
      snapshots.push({ file, contents: await readFile(file, 'utf8') });
    } catch (error) {
      if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
        snapshots.push({ file, contents: undefined });
      } else {
        throw error;
      }
    }
  }
  return snapshots;
}

async function restoreSnapshots(snapshots: FileSnapshot[]): Promise<void> {
  for (const snapshot of [...snapshots].reverse()) {
    if (snapshot.contents === undefined) await rm(snapshot.file, { force: true });
    else await atomicWriteText(snapshot.file, snapshot.contents, { mode: 0o600 });
  }
}

function validateAgentId(value: string | undefined): string {
  if (value === undefined || !/^[A-Za-z0-9_-]{1,160}$/.test(value)) {
    throw new CliError(
      '--agent-id is required and must use 1-160 letters, numbers, dashes, or underscores',
      2,
    );
  }
  return value;
}

function validateUrl(value: string): string {
  if (value.length > 2_048) throw new CliError('--url is too long', 2);
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new CliError('--url must be an absolute HTTP(S) URL', 2);
  }
  if (!['http:', 'https:'].includes(url.protocol) || url.username !== '' || url.password !== '') {
    throw new CliError('--url must be an HTTP(S) URL without embedded credentials', 2);
  }
  if (url.search !== '' || url.hash !== '') {
    throw new CliError('--url must not contain query parameters or a fragment', 2);
  }
  return url.toString().replace(/\/$/, '');
}

async function checkHealth(url: string): Promise<{
  checked: boolean;
  reachable: boolean;
  note: string;
}> {
  try {
    const response = await fetch(`${url.replace(/\/$/, '')}/health`, {
      signal: AbortSignal.timeout(3_000),
    });
    return {
      checked: true,
      reachable: response.ok,
      note: `HTTP ${response.status}`,
    };
  } catch {
    return { checked: true, reachable: false, note: 'request failed' };
  }
}

function printResult<T extends object>(json: boolean, value: T, context: CommandContext): void {
  if (json) {
    context.stdout(JSON.stringify(value, null, 2));
    return;
  }
  for (const [key, item] of Object.entries(value)) {
    context.stdout(`${key}: ${typeof item === 'string' ? item : JSON.stringify(item)}`);
  }
}

function restartInstructions(statuses: HostStatus[]): string[] {
  return statuses.flatMap((status) => (status.remediation === null ? [] : [status.remediation]));
}

function isDegraded(status: HostStatus): boolean {
  return (
    !status.installed ||
    !status.runtimePresent ||
    status.compatibility !== 'supported' ||
    status.activation === 'inactive' ||
    status.activation === 'trust_required' ||
    status.activation === 'unknown'
  );
}

function uniqueTargets(targets: HostId[]): HostId[] {
  return HOST_IDS.filter((target) => targets.includes(target));
}

function sha256(value: string): string {
  return createHash('sha256').update(value).digest('hex');
}
