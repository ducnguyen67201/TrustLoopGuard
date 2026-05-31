import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const demoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
let loaded = false;

export function loadDemoEnvForCurrentScript(): void {
  if (loaded) return;
  loaded = true;

  const scriptPath = process.argv[1] ?? '';
  const envFiles = [resolve(demoRoot, '.env')];

  if (scriptPath.includes('/demo/proxy/')) {
    envFiles.push(resolve(demoRoot, 'proxy', '.env'));
  }
  if (scriptPath.includes('/demo/raw-agent/')) {
    envFiles.push(resolve(demoRoot, 'raw-agent', '.env'));
  }

  for (const filePath of envFiles) {
    loadEnvFile(filePath);
  }
}

function loadEnvFile(filePath: string): void {
  if (!existsSync(filePath)) return;

  for (const line of readFileSync(filePath, 'utf8').split(/\r?\n/)) {
    const parsed = parseEnvLine(line);
    if (parsed === null || process.env[parsed.key] !== undefined) continue;
    process.env[parsed.key] = parsed.value;
  }
}

function parseEnvLine(line: string): { key: string; value: string } | null {
  const trimmed = line.trim();
  if (trimmed === '' || trimmed.startsWith('#')) return null;

  const separator = trimmed.indexOf('=');
  if (separator <= 0) return null;

  const key = trimmed.slice(0, separator).trim();
  const rawValue = trimmed.slice(separator + 1).trim();
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key)) return null;

  return { key, value: unquote(rawValue) };
}

function unquote(value: string): string {
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    return value.slice(1, -1);
  }

  return value;
}
