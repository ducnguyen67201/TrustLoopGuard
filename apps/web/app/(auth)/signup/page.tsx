import Link from 'next/link';
import { redirect } from 'next/navigation';

import { getAuthCapabilities, hasOAuthProvider } from '@/lib/auth-capabilities';

import { AuthScreen } from '../auth-screen';
import { OrDivider } from '../form-feedback';
import { OAuthButtons } from '../signin/oauth-buttons';
import { SignupForm } from './signup-form';

export default async function SignUpPage({
  searchParams,
}: {
  searchParams: Promise<{ callbackUrl?: string | string[] }>;
}) {
  const params = await searchParams;
  const callbackUrl = safeRedirect(
    Array.isArray(params.callbackUrl) ? params.callbackUrl[0] : params.callbackUrl,
  );
  const authCapabilities = getAuthCapabilities();
  const oauthConfigured = hasOAuthProvider(authCapabilities.oauthProviders);

  if (!authCapabilities.credentials) {
    redirect(callbackUrl === '/' ? '/signin' : `/signin?callbackUrl=${encodeURIComponent(callbackUrl)}`);
  }

  return (
    <AuthScreen
      eyebrow="Safety checks for your AI apps"
      title="Let's get you set up."
      description="TrustLoopGuard adds safety checks to your AI apps — it can block, rewrite, or flag risky requests automatically. Create an account and you'll be ready to add your first rule in minutes."
      cardTitle="Create your account"
      cardDescription="Takes about a minute. Pick whichever option is easiest for you."
    >
      {oauthConfigured ? (
        <div className="space-y-2">
          <OAuthButtons callbackUrl={callbackUrl} providers={authCapabilities.oauthProviders} />
          <p className="text-xs text-muted-foreground">
            Fastest way to start — there&apos;s no password to set up.
          </p>
        </div>
      ) : null}
      {oauthConfigured ? <OrDivider /> : null}
      <div className="space-y-3">
        {oauthConfigured ? (
          <p className="text-sm font-medium text-foreground">
            Or create a username and password
          </p>
        ) : null}
        <SignupForm callbackUrl={callbackUrl} />
      </div>
      <p className="text-center text-sm text-muted-foreground">
        Already have an account?{' '}
        <Link
          href="/signin"
          className="font-medium text-foreground underline-offset-4 hover:underline focus-visible:rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          Sign in
        </Link>
      </p>
    </AuthScreen>
  );
}

function safeRedirect(value: string | undefined | null): string {
  if (typeof value !== 'string' || value.trim() === '') return '/';
  if (!value.startsWith('/') || value.startsWith('//')) return '/';
  if (value.startsWith('/signin') || value.startsWith('/signup') || value.startsWith('/api/auth')) {
    return '/';
  }
  return value;
}
