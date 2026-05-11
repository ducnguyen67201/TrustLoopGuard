import 'server-only';

import { eq } from 'drizzle-orm';
import Credentials from 'next-auth/providers/credentials';
import { z } from 'zod';

import { db } from '@/lib/db/client';
import { users } from '@/lib/db/schema/auth';

import { verifyPassword } from './password';

const credentialsSchema = z.object({
  email: z.string().email(),
  password: z.string().min(1),
});

export const credentialsProvider = Credentials({
  name: 'Email',
  credentials: {
    email: { label: 'Email', type: 'email' },
    password: { label: 'Password', type: 'password' },
  },
  async authorize(raw) {
    const parsed = credentialsSchema.safeParse(raw);
    if (!parsed.success) return null;

    const { email, password } = parsed.data;
    const [user] = await db
      .select()
      .from(users)
      .where(eq(users.email, email))
      .limit(1);
    if (!user?.passwordHash) return null;

    const valid = await verifyPassword(password, user.passwordHash);
    if (!valid) return null;

    return { id: user.id, email: user.email, name: user.name ?? null };
  },
});
