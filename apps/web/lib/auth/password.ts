import 'server-only';

import { hash, verify } from '@node-rs/argon2';

const PARAMS = {
  timeCost: 3,
  memoryCost: 19456,
  parallelism: 1,
} as const;

export async function hashPassword(plain: string): Promise<string> {
  return hash(plain, PARAMS);
}

export async function verifyPassword(plain: string, stored: string): Promise<boolean> {
  try {
    return await verify(stored, plain);
  } catch {
    return false;
  }
}
