import type { MediaAdapter } from 'fumadocs-openapi';

type MediaBody = Parameters<MediaAdapter['encode']>[0]['body'];

function scalarToYaml(value: MediaBody): string {
  if (value === null) {
    return 'null';
  }

  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }

  return JSON.stringify(String(value));
}

function toYaml(value: MediaBody, depth = 0): string {
  const indent = '  '.repeat(depth);
  const nextIndent = '  '.repeat(depth + 1);

  if (Array.isArray(value)) {
    if (value.length === 0) {
      return '[]';
    }

    return value
      .map((item) => {
        if (item !== null && typeof item === 'object') {
          return `${indent}-\n${toYaml(item, depth + 1)}`;
        }

        return `${indent}- ${scalarToYaml(item)}`;
      })
      .join('\n');
  }

  if (value !== null && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, MediaBody>);

    if (entries.length === 0) {
      return '{}';
    }

    return entries
      .map(([key, item]) => {
        if (item !== null && typeof item === 'object') {
          return `${indent}${key}:\n${toYaml(item, depth + 1)}`;
        }

        return `${indent}${key}: ${scalarToYaml(item)}`;
      })
      .join('\n')
      .replaceAll(`\n${indent}${nextIndent}`, `\n${nextIndent}`);
  }

  return scalarToYaml(value);
}

export const yamlMediaAdapter: MediaAdapter = {
  encode(data) {
    return toYaml(data.body);
  },
  generateExample(data, ctx) {
    const body = toYaml(data.body);

    switch (ctx.lang) {
      case 'js':
        return `const body = ${JSON.stringify(body)};`;
      case 'python':
        return `body = ${JSON.stringify(body)}`;
      default:
        return undefined;
    }
  },
};
