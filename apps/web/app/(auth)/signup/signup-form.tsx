'use client';

import { useState, useTransition } from 'react';
import { signIn } from 'next-auth/react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PasswordInput } from '@/components/ui/password-input';
import { sha256Hex } from '@/lib/password';

import { FormError, Spinner } from '../form-feedback';

const USERNAME_RE = /^[A-Za-z0-9_.-]{3,64}$/;
const MIN_PASSWORD_LEN = 8;
const MAX_PASSWORD_LEN = 128;

interface SignupFormProps {
  callbackUrl: string;
}

export function SignupForm({ callbackUrl }: SignupFormProps) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  // Advisory-only: gentle "do these match?" feedback while the user types the
  // confirm field. Does not gate submission — the real check still runs onSubmit.
  const confirmMatch: 'idle' | 'match' | 'mismatch' =
    confirm.length === 0 ? 'idle' : confirm === password ? 'match' : 'mismatch';

  function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    if (!USERNAME_RE.test(username.trim())) {
      setError('Pick a username with 3–64 characters — letters, numbers, and _ - . are allowed.');
      return;
    }
    if (password.length < MIN_PASSWORD_LEN || password.length > MAX_PASSWORD_LEN) {
      setError(
        `Your password needs to be between ${MIN_PASSWORD_LEN} and ${MAX_PASSWORD_LEN} characters.`,
      );
      return;
    }
    if (password !== confirm) {
      setError("The two passwords don't match. Please re-enter them.");
      return;
    }

    startTransition(async () => {
      const hashed = await sha256Hex(password);

      const signupRes = await fetch('/api/signup', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: username.trim(), password: hashed }),
      });
      if (!signupRes.ok) {
        const body = (await signupRes.json().catch(() => null)) as { message?: string } | null;
        setError(body?.message ?? "We couldn't create your account just now. Please try again.");
        return;
      }

      const loginRes = await signIn('credentials', {
        username: username.trim(),
        password: hashed,
        redirect: false,
        callbackUrl,
      });
      if (!loginRes || loginRes.error) {
        // Signup succeeded but auto-login didn't — send them to the
        // sign-in page so they can complete it manually.
        window.location.href = `/signin?callbackUrl=${encodeURIComponent(callbackUrl)}`;
        return;
      }
      window.location.href = loginRes.url ?? callbackUrl;
    });
  }

  return (
    <form onSubmit={onSubmit} className="space-y-4">
      <FormError message={error} />
      <div className="grid gap-2">
        <Label htmlFor="signup-username">Username</Label>
        <Input
          id="signup-username"
          name="username"
          type="text"
          autoComplete="username"
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          disabled={pending}
          aria-invalid={Boolean(error)}
          aria-describedby="signup-username-hint"
          required
        />
        <p id="signup-username-hint" className="text-xs text-muted-foreground">
          3–64 characters: letters, numbers, and{' '}
          <span className="font-mono">_ - .</span>
        </p>
      </div>
      <div className="grid gap-2">
        <Label htmlFor="signup-password">Password</Label>
        <PasswordInput
          id="signup-password"
          name="password"
          autoComplete="new-password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          disabled={pending}
          aria-invalid={Boolean(error)}
          aria-describedby="signup-password-hint"
          required
        />
        <p id="signup-password-hint" className="text-xs text-muted-foreground">
          Use at least {MIN_PASSWORD_LEN} characters. A short phrase you&apos;ll remember works
          well.
        </p>
      </div>
      <div className="grid gap-2">
        <Label htmlFor="signup-confirm">Confirm password</Label>
        <PasswordInput
          id="signup-confirm"
          name="confirm"
          autoComplete="new-password"
          value={confirm}
          onChange={(event) => setConfirm(event.target.value)}
          disabled={pending}
          aria-describedby="signup-confirm-hint"
          required
        />
        <p
          id="signup-confirm-hint"
          aria-live="polite"
          className="min-h-4 text-xs empty:hidden"
        >
          {confirmMatch === 'match' ? (
            <span className="text-muted-foreground">Passwords match.</span>
          ) : confirmMatch === 'mismatch' ? (
            <span className="text-destructive">These don&apos;t match yet.</span>
          ) : null}
        </p>
      </div>
      <Button type="submit" className="w-full" disabled={pending}>
        {pending ? (
          <>
            <Spinner />
            Creating account…
          </>
        ) : (
          'Create account'
        )}
      </Button>
    </form>
  );
}
