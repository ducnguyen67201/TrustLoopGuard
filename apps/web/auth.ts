import NextAuth from 'next-auth';
import Credentials from 'next-auth/providers/credentials';
import GitHub from 'next-auth/providers/github';
import Google from 'next-auth/providers/google';

import { env } from '@/env';
import { getServerUrl } from '@/lib/server-url';

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

    const res = await fetch(`${getServerUrl()}/v1/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
      cache: 'no-store',
    });
    if (!res.ok) return null;

    const body = (await res.json()) as {
      user_id?: string;
      username?: string;
      jwt?: string;
    };
    if (!body.user_id || !body.username) return null;

    return {
      id: body.user_id,
      name: body.username,
      // NextAuth merges the returned `user` into the `jwt` callback's
      // `user` arg on first sign-in. We stash the Rust-issued JWT
      // there so the callback can persist it into the session token.
      tlJwt: body.jwt,
    } as { id: string; name: string; tlJwt?: string };
  },
});

const providers = [
  credentialsProvider,
  ...(env.AUTH_GOOGLE_ID && env.AUTH_GOOGLE_SECRET
    ? [
        Google({
          clientId: env.AUTH_GOOGLE_ID,
          clientSecret: env.AUTH_GOOGLE_SECRET,
        }),
      ]
    : []),
  ...(env.AUTH_GITHUB_ID && env.AUTH_GITHUB_SECRET
    ? [
        GitHub({
          clientId: env.AUTH_GITHUB_ID,
          clientSecret: env.AUTH_GITHUB_SECRET,
        }),
      ]
    : []),
];

export const { handlers, auth, signIn, signOut } = NextAuth({
  session: { strategy: 'jwt' },
  pages: { signIn: '/signin' },
  providers,
  callbacks: {
    async jwt({ token, user, account }) {
      if (account) {
        token['loginMethod'] = account.provider;
      }
      if (user?.name && !token['username']) {
        token['username'] = user.name;
      }
      // Persist the Rust-issued JWT from the credentials authorize()
      // into the session token. OAuth users (Google/GitHub) don't
      // get one — the web falls back to TL_API_KEY + header
      // forwarding for them until OAuth ↔ Rust-user binding lands.
      const incomingJwt = (user as { tlJwt?: string } | undefined)?.tlJwt;
      if (typeof incomingJwt === 'string' && incomingJwt !== '') {
        token['tlJwt'] = incomingJwt;
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
      if (typeof token?.['tlJwt'] === 'string' && session.user) {
        (session.user as { tlJwt?: string }).tlJwt = token['tlJwt'];
      }
      return session;
    },
  },
});
