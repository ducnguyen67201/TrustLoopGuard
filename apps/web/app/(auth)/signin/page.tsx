import Link from 'next/link';

import { env } from '@/env';

import { SignInForm } from './signin-form';

export default function SignInPage() {
  const credentialsEnabled = env.AUTH_ALLOW_SIGNUP;

  if (!credentialsEnabled) {
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
      <SignInForm />
      <p className="mt-6 text-sm text-[color:var(--color-text-muted)]">
        New here? <Link href="/signup" className="underline">Create an account</Link>.
      </p>
    </main>
  );
}
