import { createHash } from 'node:crypto';

function canonical(value: object | string | number | boolean | null): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map((item) => canonical(item)).join(',')}]`;
  const entries = Object.entries(value).sort(([left], [right]) => left.localeCompare(right));
  return `{${entries.map(([key, nested]) => `${JSON.stringify(key)}:${canonical(nested)}`).join(',')}}`;
}

export function schemaHash(schema: object): string {
  return `sha256:v1:${createHash('sha256').update(canonical(schema)).digest('hex')}`;
}
