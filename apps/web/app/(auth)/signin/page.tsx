import Link from 'next/link';

import { getAuthCapabilities, hasOAuthProvider } from '@/lib/auth-capabilities';

import { AuthScreen } from '../auth-screen';
import { OrDivider } from '../form-feedback';
import { CredentialsForm } from './credentials-form';
import { OAuthButtons } from './oauth-buttons';

export default async function SignInPage({
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

  return (
    <AuthScreen
      eyebrow="Safety checks for your AI apps"
      title="Welcome back."
      description="TrustLoopGuard adds safety checks to your AI apps — it can block, rewrite, or flag risky requests automatically. Sign in to see what's been happening and adjust your rules."
      cardTitle="Sign in"
      cardDescription={
        authCapabilities.credentials
          ? 'Pick the fastest way to get in below.'
          : 'Continue with one of the options below.'
      }
    >
      {oauthConfigured ? (
        <div className="space-y-2">
          <OAuthButtons callbackUrl={callbackUrl} providers={authCapabilities.oauthProviders} />
          <p className="text-xs text-muted-foreground">
            Fastest way in — there&apos;s no extra password to remember.
          </p>
        </div>
      ) : null}
      {oauthConfigured && authCapabilities.credentials ? <OrDivider /> : null}
      {authCapabilities.credentials ? (
        <div className="space-y-3">
          {oauthConfigured ? (
            <p className="text-sm font-medium text-foreground">
              Or sign in with a username and password
            </p>
          ) : null}
          <CredentialsForm callbackUrl={callbackUrl} />
        </div>
      ) : null}
      {!oauthConfigured && !authCapabilities.credentials ? (
        <p className="text-sm text-muted-foreground">
          This deployment doesn&apos;t have a sign-in method set up yet. Ask whoever set up
          TrustLoopGuard to add one.
        </p>
      ) : null}
      <p className="text-center text-sm text-muted-foreground">
        New here?{' '}
        <Link
          href="/signup"
          className="font-medium text-foreground underline-offset-4 hover:underline focus-visible:rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          Create an account
        </Link>
      </p>
    </AuthScreen>
  );
}

function safeRedirect(value: FormDataEntryValue | string | undefined | null): string {
  if (typeof value !== 'string' || value.trim() === '') return '/';
  if (!value.startsWith('/') || value.startsWith('//')) return '/';
  if (value.startsWith('/signin') || value.startsWith('/api/auth')) return '/';
  return value;
}
