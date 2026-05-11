'use server';

import { headers } from 'next/headers';
import { redirect } from 'next/navigation';
import { eq } from 'drizzle-orm';
import { z } from 'zod';

import { env } from '@/env';
import { db } from '@/lib/db/client';
import { users } from '@/lib/db/schema/auth';
import { hashPassword } from '@/lib/auth/password';
import { check } from '@/lib/auth/rate-limit';
import { signIn } from '@/auth';

const signupSchema = z.object({
  email: z.string().email(),
  password: z.string().min(12, 'Password must be at least 12 characters'),
  name: z.string().min(1).max(120).optional(),
});

export type SignupState = { error?: string };

export async function signupAction(
  _prev: SignupState,
  formData: FormData,
): Promise<SignupState> {
  if (!env.AUTH_ALLOW_SIGNUP) {
    return { error: 'Sign-up is disabled on this deployment.' };
  }

  const headerList = await headers();
  const ip =
    headerList.get('x-forwarded-for')?.split(',')[0]?.trim() ??
    headerList.get('x-real-ip') ??
    'unknown';

  const rl = check(`signup:${ip}`);
  if (!rl.ok) {
    return {
      error: `Too many sign-up attempts. Try again in ${rl.retryAfterSeconds}s.`,
    };
  }

  const parsed = signupSchema.safeParse({
    email: formData.get('email'),
    password: formData.get('password'),
    name: formData.get('name') || undefined,
  });
  if (!parsed.success) {
    return { error: parsed.error.issues[0]?.message ?? 'Invalid input' };
  }

  const { email, password, name } = parsed.data;

  const [existing] = await db
    .select({ id: users.id })
    .from(users)
    .where(eq(users.email, email))
    .limit(1);
  if (existing) {
    return { error: 'Email already registered.' };
  }

  const passwordHash = await hashPassword(password);
  await db.insert(users).values({
    email,
    passwordHash,
    ...(name ? { name } : {}),
  });

  await signIn('credentials', {
    email,
    password,
    redirect: false,
  });

  redirect('/dashboard');
}
