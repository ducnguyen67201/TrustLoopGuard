import NextAuth from 'next-auth';
import Credentials from 'next-auth/providers/credentials';
import GitHub from 'next-auth/providers/github';
import Google from 'next-auth/providers/google';

import { env } from '@/env';
import { isCredentialsAuthEnabled } from '@/lib/auth-capabilities';
import { safeAuthRedirect } from '@/lib/auth-redirect';
import { getServerUrl } from '@/lib/server-url';

process.env['AUTH_URL'] ??= env.AUTH_URL;
process.env['NEXTAUTH_URL'] ??= env.AUTH_URL;

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
  ...(isCredentialsAuthEnabled() ? [credentialsProvider] : []),
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
  trustHost: true,
  session: { strategy: 'jwt' },
  pages: { signIn: '/signin' },
  providers,
  callbacks: {
    async redirect({ url }) {
      return safeAuthRedirect(url, {
        appUrl: env.AUTH_URL,
        serverUrl: env.TL_SERVER_URL,
        publicServerUrl: env.NEXT_PUBLIC_TL_SERVER_URL,
      });
    },
    async jwt({ token, user, account }) {
      if (account) {
        token['loginMethod'] = account.provider;
      }
      if (
        account &&
        (account.provider === 'google' || account.provider === 'github') &&
        account.providerAccountId
      ) {
        const linked = await linkOAuthIdentity({
          provider: account.provider,
          providerSubject: account.providerAccountId,
          email: user.email,
          name: user.name,
        });
        token.sub = linked.user_id;
        token['username'] = linked.username;
        if (linked.jwt) {
          token['tlJwt'] = linked.jwt;
        }
      }
      if (user?.name && !token['username']) {
        token['username'] = user.name;
      }
      // Persist the Rust-issued JWT from the credentials authorize()
      // or OAuth identity-link response into the session token.
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

type OAuthLinkResponse = {
  user_id: string;
  username: string;
  jwt?: string;
};

async function linkOAuthIdentity({
  provider,
  providerSubject,
  email,
  name,
}: {
  provider: 'google' | 'github';
  providerSubject: string;
  email?: string | null | undefined;
  name?: string | null | undefined;
}): Promise<OAuthLinkResponse> {
  const cleanEmail = email?.trim();
  if (!cleanEmail) {
    throw new Error(`${provider} did not return an email address`);
  }
  const key = env.TL_API_KEY?.trim();
  if (!key) {
    throw new Error('TL_API_KEY is required to link OAuth identity');
  }
  const res = await fetch(`${getServerUrl()}/v1/identity/oauth-session`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${key}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      provider,
      provider_subject: providerSubject,
      email: cleanEmail,
      name: name?.trim() || undefined,
    }),
    cache: 'no-store',
  });
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new Error(`OAuth identity link failed (${res.status}): ${text}`);
  }
  return (await res.json()) as OAuthLinkResponse;
}
