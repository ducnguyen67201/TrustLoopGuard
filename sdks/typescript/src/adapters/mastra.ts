import type { ToolAdapterCandidate, ToolFramework, ToolRuntimeValue } from '../tool-discovery.js';

export function normalizeMastraTool(
  owner: object,
  registryKey: string,
  replaceOwner: (replacement: object) => boolean,
  framework: ToolFramework = 'mastra',
): ToolAdapterCandidate | undefined {
  const execute = Reflect.get(owner, 'execute', owner);
  if (typeof execute !== 'function') return undefined;

  const id = stringProperty(owner, 'id');
  const name = id ?? stringProperty(owner, 'name') ?? registryKey;
  if (name.length === 0) return undefined;

  const candidate: ToolAdapterCandidate = {
    framework,
    registryKey,
    name,
    execute,
    owner,
    replaceOwner,
  };
  const description = stringProperty(owner, 'description');
  if (description !== undefined) candidate.description = description;
  const inputSchema = objectProperty(owner, 'inputSchema') ?? objectProperty(owner, 'parameters');
  if (inputSchema !== undefined) candidate.inputSchema = inputSchema;
  const outputSchema = objectProperty(owner, 'outputSchema');
  if (outputSchema !== undefined) candidate.outputSchema = outputSchema;
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
