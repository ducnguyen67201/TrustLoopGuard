import type { ToolAdapterCandidate, ToolRuntimeValue } from '../tool-discovery.js';

export function normalizeOpenAiAgentsTool(
  owner: object,
  registryKey: string,
  replaceOwner: (replacement: object) => boolean,
): ToolAdapterCandidate | undefined {
  const execute = Reflect.get(owner, 'execute', owner);
  const name = stringProperty(owner, 'name');
  if (typeof execute !== 'function' || name === undefined) return undefined;

  const candidate: ToolAdapterCandidate = {
    framework: 'openai-agents',
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
