import NextAuth from 'next-auth';
import Google from 'next-auth/providers/google';
import { DrizzleAdapter } from '@auth/drizzle-adapter';

import { env } from '@/env';
import { db } from '@/lib/db/client';
import { credentialsProvider } from '@/lib/auth/credentials';

const providers = [];

if (env.AUTH_GOOGLE_ID && env.AUTH_GOOGLE_SECRET) {
  providers.push(
    Google({
      clientId: env.AUTH_GOOGLE_ID,
      clientSecret: env.AUTH_GOOGLE_SECRET,
      // Safe because the signIn callback below blocks unverified Google
      // emails before account linking runs.
      allowDangerousEmailAccountLinking: true,
    }),
  );
}

if (env.AUTH_ALLOW_SIGNUP) {
  providers.push(credentialsProvider);
}

export const { handlers, auth, signIn, signOut } = NextAuth({
  adapter: DrizzleAdapter(db),
  session: { strategy: 'jwt' },
  trustHost: env.AUTH_TRUST_HOST,
  pages: { signIn: '/signin' },
  providers,
  callbacks: {
    async session({ session, token }) {
      if (token?.sub && session.user) {
        session.user.id = token.sub;
      }
      return session;
    },
    async signIn({ account, profile }) {
      if (account?.provider !== 'google') return true;

      const email = profile?.email;
      const emailVerified = (profile as { email_verified?: boolean } | undefined)
        ?.email_verified;
      if (!email || emailVerified !== true) {
        return '/signin?error=verify-google-email';
      }

      return true;
    },
  },
});
