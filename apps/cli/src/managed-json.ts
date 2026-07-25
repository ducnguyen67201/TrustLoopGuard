import { constants } from 'node:fs';
import {
  chmod,
  copyFile,
  lstat,
  mkdir,
  open,
  readFile,
  rename,
  rm,
  writeFile,
} from 'node:fs/promises';
import { dirname } from 'node:path';
import { randomUUID } from 'node:crypto';

import type { JsonObject, JsonValue } from './types.js';

export function isJsonObject(value: JsonValue | undefined): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export async function readJsonValue(file: string): Promise<JsonValue | undefined> {
  let text: string;
  try {
    text = await readFile(file, 'utf8');
  } catch (error) {
    if (error instanceof Error && isNodeError(error) && error.code === 'ENOENT') return undefined;
    throw error;
  }
  if (text.trim() === '') throw new Error(`${file} is empty; fix or remove it before retrying`);
  try {
    return JSON.parse(text) as JsonValue;
  } catch {
    throw new Error(`${file} contains malformed JSON; fix it before retrying`);
  }
}

export async function readJsonObject(file: string): Promise<JsonObject | undefined> {
  const value = await readJsonValue(file);
  if (value === undefined) return undefined;
  if (!isJsonObject(value)) throw new Error(`${file} must contain a JSON object`);
  return value;
}

export async function atomicWriteText(
  file: string,
  text: string,
  options: { backup?: boolean; mode?: number } = {},
): Promise<void> {
  await mkdir(dirname(file), { recursive: true });
  await rejectSymlink(file);
  if (options.backup === true) await createBackupOnce(file);
  const temporary = `${file}.${randomUUID()}.tmp`;
  try {
    await writeFile(temporary, text, {
      encoding: 'utf8',
      flag: 'wx',
      mode: options.mode ?? 0o600,
    });
    await rename(temporary, file);
    if (process.platform !== 'win32') await chmod(file, options.mode ?? 0o600);
  } catch (error) {
    await rm(temporary, { force: true });
    throw error;
  }
}

export async function atomicWriteJson(
  file: string,
  value: JsonValue,
  options: { backup?: boolean } = {},
): Promise<void> {
  await atomicWriteText(file, `${JSON.stringify(value, null, 2)}\n`, {
    backup: options.backup ?? false,
    mode: 0o600,
  });
}

async function createBackupOnce(file: string): Promise<string | undefined> {
  try {
    await lstat(file);
  } catch (error) {
    if (error instanceof Error && isNodeError(error) && error.code === 'ENOENT') return undefined;
    throw error;
  }
  const backup = `${file}.tlg.bak`;
  try {
    await copyFile(file, backup, constants.COPYFILE_EXCL);
    if (process.platform !== 'win32') await chmod(backup, 0o600);
  } catch (error) {
    if (!(error instanceof Error) || !isNodeError(error) || error.code !== 'EEXIST') throw error;
  }
  return backup;
}

export async function withFileLock<T>(
  lockFile: string,
  action: () => Promise<T>,
  timeoutMs = 2_000,
): Promise<T> {
  await mkdir(dirname(lockFile), { recursive: true, mode: 0o700 });
  const deadline = Date.now() + timeoutMs;
  let handle;
  while (handle === undefined) {
    try {
      handle = await open(lockFile, 'wx', 0o600);
    } catch (error) {
      if (
        !(error instanceof Error) ||
        !isNodeError(error) ||
        error.code !== 'EEXIST' ||
        Date.now() >= deadline
      ) {
        throw new Error(`could not acquire installer lock ${lockFile}`);
      }
      await new Promise((resolve) => setTimeout(resolve, 25));
    }
  }
  try {
    await handle.writeFile(`${process.pid}\n`);
    return await action();
  } finally {
    await handle.close();
    await rm(lockFile, { force: true });
  }
}

export async function rejectSymlink(file: string): Promise<void> {
  try {
    const stats = await lstat(file);
    if (stats.isSymbolicLink()) throw new Error(`${file} must not be a symbolic link`);
  } catch (error) {
    if (error instanceof Error && isNodeError(error) && error.code === 'ENOENT') return;
    throw error;
  }
}

function isNodeError(error: Error): error is NodeJS.ErrnoException {
  return 'code' in error;
}
