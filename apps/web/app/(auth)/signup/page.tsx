import { notFound } from 'next/navigation';

import { env } from '@/env';

import { SignupForm } from './signup-form';

export default function SignUpPage() {
  if (!env.AUTH_ALLOW_SIGNUP) {
    notFound();
  }
  return (
    <main className="mx-auto max-w-md px-6 py-16">
      <h1 className="text-2xl font-semibold tracking-tight">Create an account</h1>
      <p className="mt-3 text-sm text-[color:var(--color-text-muted)]">
        Use an email and a password of at least 12 characters.
      </p>
      <SignupForm />
    </main>
  );
}
