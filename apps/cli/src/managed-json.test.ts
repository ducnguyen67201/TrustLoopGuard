import { mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, describe, expect, test } from 'vitest';

import { atomicWriteJson, readJsonObject, rejectSymlink, withFileLock } from './managed-json.js';

const directories: string[] = [];

async function temporaryDirectory(): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), 'tlg-managed-json-'));
  directories.push(directory);
  return directory;
}

afterEach(async () => {
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true })));
});

describe('managed JSON', () => {
  test('reads absent files and rejects malformed or array-valued JSON', async () => {
    const directory = await temporaryDirectory();
    expect(await readJsonObject(join(directory, 'missing.json'))).toBeUndefined();
    await writeFile(join(directory, 'bad.json'), '{');
    await expect(readJsonObject(join(directory, 'bad.json'))).rejects.toThrow(/malformed/);
    await writeFile(join(directory, 'array.json'), '[]');
    await expect(readJsonObject(join(directory, 'array.json'))).rejects.toThrow(/object/);
  });

  test('writes atomically and creates only one recovery backup', async () => {
    const directory = await temporaryDirectory();
    const file = join(directory, 'settings.json');
    await writeFile(file, '{"foreign":1}\n');
    await atomicWriteJson(file, { foreign: 1, managed: true }, { backup: true });
    await atomicWriteJson(file, { foreign: 2, managed: true }, { backup: true });
    expect(await readFile(`${file}.tlg.bak`, 'utf8')).toBe('{"foreign":1}\n');
    expect(await readJsonObject(file)).toEqual({ foreign: 2, managed: true });
  });

  test('serializes concurrent actions through a bounded lock', async () => {
    const directory = await temporaryDirectory();
    const lock = join(directory, '.lock');
    const order: string[] = [];
    await Promise.all([
      withFileLock(lock, async () => {
        order.push('first-start');
        await new Promise((resolve) => setTimeout(resolve, 30));
        order.push('first-end');
      }),
      new Promise<void>((resolve, reject) => {
        setTimeout(() => {
          withFileLock(lock, async () => {
            order.push('second');
          }).then(resolve, reject);
        }, 5);
      }),
    ]);
    expect(order).toEqual(['first-start', 'first-end', 'second']);
  });

  test('rejects symbolic-link write targets', async () => {
    const directory = await temporaryDirectory();
    const target = join(directory, 'target.json');
    const link = join(directory, 'link.json');
    await writeFile(target, '{}');
    await symlink(target, link);
    await expect(rejectSymlink(link)).rejects.toThrow(/symbolic link/);
  });
});
