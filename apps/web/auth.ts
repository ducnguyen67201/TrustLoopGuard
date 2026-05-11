import NextAuth from 'next-auth';
import { DrizzleAdapter } from '@auth/drizzle-adapter';

import { env } from '@/env';
import { db } from '@/lib/db/client';
import { credentialsProvider } from '@/lib/auth/credentials';

const providers = [];
if (env.AUTH_ALLOW_SIGNUP) {
  providers.push(credentialsProvider);
}

export const { handlers, auth, signIn, signOut } = NextAuth({
  adapter: DrizzleAdapter(db),
  session: { strategy: 'jwt' },
  pages: { signIn: '/signin' },
  providers,
  callbacks: {
    async session({ session, token }) {
      if (token?.sub && session.user) {
        session.user.id = token.sub;
      }
      return session;
    },
  },
});
