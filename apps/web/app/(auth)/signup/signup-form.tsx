'use client';

import { useState, useTransition } from 'react';
import Link from 'next/link';
import { signIn } from 'next-auth/react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PasswordInput } from '@/components/ui/password-input';
import { sha256Hex } from '@/lib/password';

const USERNAME_RE = /^[A-Za-z0-9_.-]{3,64}$/;
const MIN_PASSWORD_LEN = 8;
const MAX_PASSWORD_LEN = 128;

interface SignupFormProps {
  callbackUrl: string;
  /// When set, the username field is rendered read-only and the
  /// invite_token travels with the signup request so the new
  /// account is atomically joined to the invited workspace.
  inviteToken?: string;
  presetUsername?: string;
}

export function SignupForm({ callbackUrl, inviteToken, presetUsername }: SignupFormProps) {
  const [username, setUsername] = useState(presetUsername ?? '');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    if (!USERNAME_RE.test(username.trim())) {
      setError('Username must be 3-64 characters: letters, numbers, _, -, .');
      return;
    }
    if (password.length < MIN_PASSWORD_LEN || password.length > MAX_PASSWORD_LEN) {
      setError(`Password must be ${MIN_PASSWORD_LEN}-${MAX_PASSWORD_LEN} characters`);
      return;
    }
    if (password !== confirm) {
      setError('Passwords do not match');
      return;
    }

    startTransition(async () => {
      const hashed = await sha256Hex(password);

      const signupBody: Record<string, string> = {
        username: username.trim(),
        password: hashed,
      };
      if (inviteToken !== undefined && inviteToken.trim() !== '') {
        signupBody['invite_token'] = inviteToken.trim();
      }
      const signupRes = await fetch('/api/signup', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(signupBody),
      });
      if (!signupRes.ok) {
        const body = (await signupRes.json().catch(() => null)) as { message?: string } | null;
        setError(body?.message ?? 'Signup failed');
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
    <form onSubmit={onSubmit} className="space-y-3">
      <div className="grid gap-1.5">
        <Label htmlFor="signup-username">Username</Label>
        <Input
          id="signup-username"
          name="username"
          type="text"
          autoComplete="username"
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          disabled={pending || (presetUsername !== undefined && presetUsername !== '')}
          readOnly={presetUsername !== undefined && presetUsername !== ''}
          required
        />
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor="signup-password">Password</Label>
        <PasswordInput
          id="signup-password"
          name="password"
          autoComplete="new-password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          disabled={pending}
          required
        />
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor="signup-confirm">Confirm password</Label>
        <PasswordInput
          id="signup-confirm"
          name="confirm"
          autoComplete="new-password"
          value={confirm}
          onChange={(event) => setConfirm(event.target.value)}
          disabled={pending}
          required
        />
      </div>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      <Button type="submit" className="w-full" disabled={pending}>
        {pending ? 'Creating account…' : 'Create account'}
      </Button>
      <p className="text-center text-sm text-muted-foreground">
        Already have an account?{' '}
        <Link href="/signin" className="font-medium text-foreground underline">
          Sign in
        </Link>
      </p>
    </form>
  );
}
