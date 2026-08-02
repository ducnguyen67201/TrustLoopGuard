import { isAbsolute, relative } from 'node:path';

import { atomicWriteJson, isJsonObject, readJsonValue } from './managed-json.js';
import {
  HOST_IDS,
  type HostId,
  type JsonValue,
  type ProjectRegistration,
  type Registry,
} from './types.js';

const REGISTRY_VERSION = 1;
export const RUNTIME_VERSION = 'featherlane-ai-tool-gate-v1';

export function emptyRegistry(): Registry {
  return { version: REGISTRY_VERSION, projects: [] };
}

export async function readRegistry(file: string): Promise<Registry> {
  const value = await readJsonValue(file);
  if (value === undefined) return emptyRegistry();
  return parseRegistry(value, file);
}

export function parseRegistry(value: JsonValue, source = 'registry'): Registry {
  if (!isJsonObject(value) || value['version'] !== REGISTRY_VERSION) {
    throw new Error(`${source} has an unsupported Featherlane AI registry version`);
  }
  const projects = value['projects'];
  if (!Array.isArray(projects)) throw new Error(`${source} has an invalid projects list`);
  return {
    version: REGISTRY_VERSION,
    projects: projects.map((project, index) =>
      parseRegistration(project, `${source} project ${index}`),
    ),
  };
}

function parseRegistration(value: JsonValue, source: string): ProjectRegistration {
  if (!isJsonObject(value)) throw new Error(`${source} must be an object`);
  const root = stringField(value, 'root', source);
  const url = stringField(value, 'url', source);
  const agentId = stringField(value, 'agentId', source);
  const cliVersion = stringField(value, 'cliVersion', source);
  const runtimeVersion = stringField(value, 'runtimeVersion', source);
  const createdAt = stringField(value, 'createdAt', source);
  const updatedAt = stringField(value, 'updatedAt', source);
  const rawTargets = value['targets'];
  if (
    !Array.isArray(rawTargets) ||
    rawTargets.some((target) => typeof target !== 'string' || !HOST_IDS.includes(target as HostId))
  ) {
    throw new Error(`${source} has invalid targets`);
  }
  return {
    root,
    url,
    agentId,
    targets: [...new Set(rawTargets as HostId[])],
    cliVersion,
    runtimeVersion,
    createdAt,
    updatedAt,
  };
}

function stringField(value: Record<string, JsonValue>, key: string, source: string): string {
  const field = value[key];
  if (typeof field !== 'string' || field.trim() === '') {
    throw new Error(`${source} has an invalid ${key}`);
  }
  return field;
}

export async function writeRegistry(file: string, registry: Registry): Promise<void> {
  await atomicWriteJson(
    file,
    {
      version: registry.version,
      projects: registry.projects.map((project) => ({
        root: project.root,
        url: project.url,
        agentId: project.agentId,
        targets: project.targets,
        cliVersion: project.cliVersion,
        runtimeVersion: project.runtimeVersion,
        createdAt: project.createdAt,
        updatedAt: project.updatedAt,
      })),
    },
    { backup: true },
  );
}

export function findRegistration(
  registry: Registry,
  candidatePath: string,
): ProjectRegistration | undefined {
  return registry.projects
    .filter((project) => containsPath(project.root, candidatePath))
    .sort((left, right) => right.root.length - left.root.length)[0];
}

export function containsPath(root: string, candidatePath: string): boolean {
  const child = relative(root, candidatePath);
  return (
    child === '' ||
    (!child.startsWith(`..${separatorFor(child)}`) && child !== '..' && !isAbsolute(child))
  );
}

function separatorFor(value: string): '/' | '\\' {
  return value.includes('\\') ? '\\' : '/';
}

export function upsertRegistration(
  registry: Registry,
  registration: Omit<ProjectRegistration, 'createdAt' | 'updatedAt'>,
  now: string,
): Registry {
  const existing = registry.projects.find((project) => project.root === registration.root);
  const next: ProjectRegistration = {
    ...registration,
    createdAt: existing?.createdAt ?? now,
    updatedAt: now,
  };
  return {
    version: REGISTRY_VERSION,
    projects: [
      ...registry.projects.filter((project) => project.root !== registration.root),
      next,
    ].sort((left, right) => left.root.localeCompare(right.root)),
  };
}

export function removeRegistrationTargets(
  registry: Registry,
  projectRoot: string,
  targets: HostId[] | 'all',
  now: string,
): Registry {
  const projects = registry.projects.flatMap((project) => {
    if (project.root !== projectRoot) return [project];
    const retained =
      targets === 'all' ? [] : project.targets.filter((target) => !targets.includes(target));
    if (retained.length === 0) return [];
    return [{ ...project, targets: retained, updatedAt: now }];
  });
  return { version: REGISTRY_VERSION, projects };
}

export function registeredTargets(registry: Registry): Set<HostId> {
  return new Set(registry.projects.flatMap((project) => project.targets));
}
