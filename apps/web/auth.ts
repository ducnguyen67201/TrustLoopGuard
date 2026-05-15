import NextAuth from 'next-auth';
import { DrizzleAdapter } from '@auth/drizzle-adapter';
import Credentials from 'next-auth/providers/credentials';
import GitHub from 'next-auth/providers/github';
import Google from 'next-auth/providers/google';

import { env } from '@/env';
import { getDb } from '@/lib/db/client';
import { accounts, sessions, users, verificationTokens } from '@/lib/db/schema/auth';

// Credentials provider for self-hosters: hands the SHA-256-hex
// password off to tl-server's POST /v1/auth/login. Sessions for this
// provider are JWT-only — we don't write a row to `auth_users`, and
// `session.user.id` carries the tl-server user_id, not an auth_users
// row id.
const credentialsProvider = Credentials({
  id: 'credentials',
  name: 'Username',
  credentials: {
    username: { label: 'Username', type: 'text' },
    password: { label: 'Password (SHA-256 hex)', type: 'password' },
  },
  async authorize(credentials) {
    const username = typeof credentials?.username === 'string' ? credentials.username : '';
    const password = typeof credentials?.password === 'string' ? credentials.password : '';
    if (!username || !password) return null;

    const res = await fetch(`${env.NEXT_PUBLIC_TL_SERVER_URL}/v1/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
      cache: 'no-store',
    });
    if (!res.ok) return null;

    const body = (await res.json()) as { user_id?: string; username?: string };
    if (!body.user_id || !body.username) return null;

    return {
      id: body.user_id,
      name: body.username,
    };
  },
});

const providers = [
  env.AUTH_GOOGLE_ID && env.AUTH_GOOGLE_SECRET
    ? Google({
        clientId: env.AUTH_GOOGLE_ID,
        clientSecret: env.AUTH_GOOGLE_SECRET,
        allowDangerousEmailAccountLinking: true,
      })
    : null,
  env.AUTH_GITHUB_ID && env.AUTH_GITHUB_SECRET
    ? GitHub({
        clientId: env.AUTH_GITHUB_ID,
        clientSecret: env.AUTH_GITHUB_SECRET,
        allowDangerousEmailAccountLinking: true,
      })
    : null,
  credentialsProvider,
].filter((provider): provider is NonNullable<typeof provider> => provider !== null);

export const { handlers, auth, signIn, signOut } = NextAuth({
  adapter: DrizzleAdapter(getDb(), {
    usersTable: users,
    accountsTable: accounts,
    sessionsTable: sessions,
    verificationTokensTable: verificationTokens,
  }),
  session: { strategy: 'jwt' },
  pages: { signIn: '/signin' },
  providers,
  callbacks: {
    async jwt({ token, user, account }) {
      // Stamp the login method on first sign-in so the UI can show
      // the "change password" affordance only for Credentials users.
      if (account) {
        token['loginMethod'] = account.provider;
      }
      if (user?.name && !token['username']) {
        token['username'] = user.name;
      }
      return token;
    },
    async session({ session, token }) {
      if (token?.sub && session.user) {
        session.user.id = token.sub;
      }
      if (typeof token?.['loginMethod'] === 'string' && session.user) {
        (session.user as { loginMethod?: string }).loginMethod = token['loginMethod'];
      }
      if (typeof token?.['username'] === 'string' && session.user) {
        (session.user as { username?: string }).username = token['username'];
      }
      return session;
    },
  },
});
