'use server';

import { revalidatePath } from 'next/cache';
import { z } from 'zod';

import { auth } from '@/auth';
import {
  AdminApiError,
  createKey,
  revokeKey,
  type CreatedApiKey,
} from '@/lib/tl-admin/client';

const createSchema = z.object({
  name: z.string().min(1).max(80),
});

const revokeSchema = z.object({
  keyId: z.string().uuid(),
});

export type CreateKeyState =
  | { ok: true; key: CreatedApiKey }
  | { ok: false; error: string }
  | { ok: 'idle' };

export async function createKeyAction(
  _prev: CreateKeyState,
  formData: FormData,
): Promise<CreateKeyState> {
  const session = await auth();
  if (!session?.user?.id) {
    return { ok: false, error: 'Not signed in.' };
  }

  const parsed = createSchema.safeParse({ name: formData.get('name') });
  if (!parsed.success) {
    return { ok: false, error: 'Name must be 1-80 characters.' };
  }

  try {
    const key = await createKey(session.user.id, parsed.data.name);
    revalidatePath('/dashboard/keys');
    return { ok: true, key };
  } catch (err) {
    if (err instanceof AdminApiError) {
      return { ok: false, error: err.message };
    }
    return { ok: false, error: 'Failed to create key.' };
  }
}

export type RevokeKeyState = { error?: string };

export async function revokeKeyAction(
  _prev: RevokeKeyState,
  formData: FormData,
): Promise<RevokeKeyState> {
  const session = await auth();
  if (!session?.user?.id) {
    return { error: 'Not signed in.' };
  }

  const parsed = revokeSchema.safeParse({ keyId: formData.get('keyId') });
  if (!parsed.success) {
    return { error: 'Invalid key id.' };
  }

  try {
    await revokeKey(session.user.id, parsed.data.keyId);
    revalidatePath('/dashboard/keys');
    return {};
  } catch (err) {
    if (err instanceof AdminApiError) {
      return { error: err.message };
    }
    return { error: 'Failed to revoke key.' };
  }
}
