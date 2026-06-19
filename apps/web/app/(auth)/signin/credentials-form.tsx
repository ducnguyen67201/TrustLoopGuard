'use client';

import { useState, useTransition } from 'react';
import { signIn } from 'next-auth/react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PasswordInput } from '@/components/ui/password-input';
import { sha256Hex } from '@/lib/password';

import { FormError, Spinner } from '../form-feedback';

interface CredentialsFormProps {
  callbackUrl: string;
}

export function CredentialsForm({ callbackUrl }: CredentialsFormProps) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (!username.trim() || !password) {
      setError('Please fill in both your username and password.');
      return;
    }
    startTransition(async () => {
      const hashed = await sha256Hex(password);
      const res = await signIn('credentials', {
        username: username.trim(),
        password: hashed,
        redirect: false,
        callbackUrl,
      });
      if (!res || res.error) {
        setError("That username or password didn't match. Please try again.");
        return;
      }
      window.location.href = res.url ?? callbackUrl;
    });
  }

  return (
    <form onSubmit={onSubmit} className="space-y-4">
      <FormError message={error} />
      <div className="grid gap-2">
        <Label htmlFor="signin-username">Username</Label>
        <Input
          id="signin-username"
          name="username"
          type="text"
          autoComplete="username"
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          disabled={pending}
          required
        />
      </div>
      <div className="grid gap-2">
        <Label htmlFor="signin-password">Password</Label>
        <PasswordInput
          id="signin-password"
          name="password"
          autoComplete="current-password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          disabled={pending}
          required
        />
        <p className="text-xs text-muted-foreground">
          Forgot your password? Ask your workspace admin to reset it.
        </p>
      </div>
      <Button type="submit" className="w-full" disabled={pending}>
        {pending ? (
          <>
            <Spinner />
            Signing in…
          </>
        ) : (
          'Sign in'
        )}
      </Button>
    </form>
  );
}
