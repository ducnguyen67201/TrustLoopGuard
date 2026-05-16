import NextAuth from 'next-auth';
import Credentials from 'next-auth/providers/credentials';

import { env } from '@/env';

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

export const { handlers, auth, signIn, signOut } = NextAuth({
  session: { strategy: 'jwt' },
  pages: { signIn: '/signin' },
  providers: [credentialsProvider],
  callbacks: {
    async jwt({ token, user, account }) {
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
