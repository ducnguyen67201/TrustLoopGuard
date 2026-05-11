'use client';

import Link from 'next/link';
import { useActionState } from 'react';

import { signupAction, type SignupState } from './actions';

const initialState: SignupState = {};

export function SignupForm() {
  const [state, formAction, pending] = useActionState(signupAction, initialState);

  return (
    <form action={formAction} className="mt-8 space-y-4">
      <label className="block text-sm">
        <span className="block">Name (optional)</span>
        <input
          name="name"
          type="text"
          autoComplete="name"
          className="mt-1 w-full rounded border border-[color:var(--color-border)] bg-transparent px-3 py-2 text-sm"
        />
      </label>
      <label className="block text-sm">
        <span className="block">Email</span>
        <input
          name="email"
          type="email"
          required
          autoComplete="email"
          className="mt-1 w-full rounded border border-[color:var(--color-border)] bg-transparent px-3 py-2 text-sm"
        />
      </label>
      <label className="block text-sm">
        <span className="block">Password</span>
        <input
          name="password"
          type="password"
          required
          minLength={12}
          autoComplete="new-password"
          className="mt-1 w-full rounded border border-[color:var(--color-border)] bg-transparent px-3 py-2 text-sm"
        />
      </label>
      {state.error ? (
        <p className="text-sm text-red-600" role="alert">
          {state.error}
        </p>
      ) : null}
      <button
        type="submit"
        disabled={pending}
        className="w-full rounded bg-[color:var(--color-accent)] px-4 py-2 text-sm font-medium text-white disabled:opacity-60"
      >
        {pending ? 'Creating account...' : 'Create account'}
      </button>
      <p className="text-xs text-[color:var(--color-text-muted)]">
        Already have an account? <Link href="/signin" className="underline">Sign in</Link>.
      </p>
    </form>
  );
}
