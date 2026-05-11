import Link from 'next/link';

import { env } from '@/env';

import { GoogleButton } from './google-button';
import { SignInForm } from './signin-form';

export default async function SignInPage({
  searchParams,
}: {
  searchParams: Promise<{ error?: string }>;
}) {
  const credentialsEnabled = env.AUTH_ALLOW_SIGNUP;
  const googleEnabled = Boolean(env.AUTH_GOOGLE_ID && env.AUTH_GOOGLE_SECRET);
  const { error } = await searchParams;

  if (!credentialsEnabled && !googleEnabled) {
    return (
      <main className="mx-auto max-w-md px-6 py-16">
        <h1 className="text-2xl font-semibold tracking-tight">Sign in</h1>
        <p className="mt-3 text-sm text-[color:var(--color-text-muted)]">
          No sign-in methods are configured for this deployment. Set
          <code className="mx-1 font-mono">AUTH_ALLOW_SIGNUP</code>
          to enable email and password, or
          <code className="mx-1 font-mono">AUTH_GOOGLE_ID</code>
          and
          <code className="mx-1 font-mono">AUTH_GOOGLE_SECRET</code>
          to enable Google.
        </p>
      </main>
    );
  }

  return (
    <main className="mx-auto max-w-md px-6 py-16">
      <h1 className="text-2xl font-semibold tracking-tight">Sign in</h1>

      {error === 'verify-google-email' ? (
        <p className="mt-4 text-sm text-red-600" role="alert">
          Your Google email is not verified. Verify it with Google and try again.
        </p>
      ) : null}

      {googleEnabled ? (
        <div className="mt-8">
          <GoogleButton />
        </div>
      ) : null}

      {googleEnabled && credentialsEnabled ? (
        <div className="my-6 flex items-center gap-3 text-xs uppercase tracking-wider text-[color:var(--color-text-muted)]">
          <span className="h-px flex-1 bg-[color:var(--color-border)]" />
          <span>or</span>
          <span className="h-px flex-1 bg-[color:var(--color-border)]" />
        </div>
      ) : null}

      {credentialsEnabled ? <SignInForm /> : null}

      {credentialsEnabled ? (
        <p className="mt-6 text-sm text-[color:var(--color-text-muted)]">
          New here? <Link href="/signup" className="underline">Create an account</Link>.
        </p>
      ) : null}
    </main>
  );
}
