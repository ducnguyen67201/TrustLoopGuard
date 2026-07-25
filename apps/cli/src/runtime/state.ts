import { createHash, randomUUID } from 'node:crypto';
import { chmod, lstat, mkdir, readFile, readdir, rename, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import type { JsonValue } from './runtime-types.js';
import { isObject } from './wire.js';

export interface LeaseState {
  leaseId: string;
  host: 'claude' | 'codex' | 'opencode';
  sessionId: string;
  callId: string;
  url: string;
}

export interface ClaimedLease {
  state: LeaseState;
  originalFile: string;
  claimedFile: string;
}

function leaseStateKey(host: string, sessionId: string, callId: string): string {
  return createHash('sha256').update(`${host}\0${sessionId}\0${callId}`).digest('hex');
}

async function ensureStateDirectory(directory: string): Promise<void> {
  try {
    await mkdir(directory, { recursive: false, mode: 0o700 });
  } catch (error) {
    if (!(error instanceof Error) || !('code' in error) || error.code !== 'EEXIST') throw error;
  }
  const stats = await lstat(directory);
  if (!stats.isDirectory() || stats.isSymbolicLink()) {
    throw new Error('tool-gate state path is not a real directory');
  }
  if (typeof process.getuid === 'function' && stats.uid !== process.getuid()) {
    throw new Error('tool-gate state directory is owned by another user');
  }
  if (process.platform !== 'win32') await chmod(directory, 0o700);
}

export async function storeLeaseState(directory: string, state: LeaseState): Promise<void> {
  await ensureStateDirectory(directory);
  const file = join(directory, `${leaseStateKey(state.host, state.sessionId, state.callId)}.json`);
  const temporary = `${file}.${randomUUID()}.tmp`;
  await writeFile(temporary, `${JSON.stringify(state)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
  });
  await rename(temporary, file);
}

export async function claimLeaseState(
  directory: string,
  host: string,
  sessionId: string,
  callId: string,
): Promise<ClaimedLease | undefined> {
  const originalFile = join(directory, `${leaseStateKey(host, sessionId, callId)}.json`);
  return claimFile(originalFile);
}

export async function claimSessionLeaseStates(
  directory: string,
  host: string,
  sessionId: string,
): Promise<ClaimedLease[]> {
  let files: string[];
  try {
    files = await readdir(directory);
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return [];
    throw error;
  }
  const claimed: ClaimedLease[] = [];
  for (const file of files.filter((entry) => entry.endsWith('.json'))) {
    const state = await claimFile(join(directory, file));
    if (state === undefined) continue;
    if (state.state.host === host && state.state.sessionId === sessionId) {
      claimed.push(state);
    } else {
      await releaseClaim(state);
    }
  }
  return claimed;
}

async function claimFile(originalFile: string): Promise<ClaimedLease | undefined> {
  const claimedFile = `${originalFile}.claim-${randomUUID()}`;
  try {
    await rename(originalFile, claimedFile);
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
    throw error;
  }
  try {
    const value = JSON.parse(await readFile(claimedFile, 'utf8')) as JsonValue;
    return { state: parseLeaseState(value), originalFile, claimedFile };
  } catch (error) {
    await releaseClaim({ state: emptyLeaseState(), originalFile, claimedFile });
    throw error;
  }
}

export async function finishClaim(claim: ClaimedLease): Promise<void> {
  await rm(claim.claimedFile, { force: true });
}

export async function releaseClaim(claim: ClaimedLease): Promise<void> {
  try {
    await rename(claim.claimedFile, claim.originalFile);
  } catch (error) {
    if (!(error instanceof Error) || !('code' in error) || error.code !== 'ENOENT') throw error;
  }
}

function parseLeaseState(value: JsonValue): LeaseState {
  if (!isObject(value)) throw new Error('lease state must be an object');
  const host = value['host'];
  if (host !== 'claude' && host !== 'codex' && host !== 'opencode') {
    throw new Error('lease state has invalid host');
  }
  return {
    leaseId: field(value, 'leaseId'),
    host,
    sessionId: field(value, 'sessionId'),
    callId: field(value, 'callId'),
    url: field(value, 'url'),
  };
}

function field(value: Record<string, JsonValue>, key: string): string {
  const candidate = value[key];
  if (typeof candidate !== 'string' || candidate === '') {
    throw new Error(`lease state has invalid ${key}`);
  }
  return candidate;
}

function emptyLeaseState(): LeaseState {
  return { leaseId: '', host: 'claude', sessionId: '', callId: '', url: '' };
}
