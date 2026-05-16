'use client';

import { useState, useTransition } from 'react';
import Link from 'next/link';
import { signIn } from 'next-auth/react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PasswordInput } from '@/components/ui/password-input';
import { sha256Hex } from '@/lib/password';

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
      setError('Username and password are required');
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
        setError('Invalid username or password');
        return;
      }
      window.location.href = res.url ?? callbackUrl;
    });
  }

  return (
    <form onSubmit={onSubmit} className="space-y-3">
      <div className="grid gap-1.5">
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
      <div className="grid gap-1.5">
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
      </div>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      <Button type="submit" className="w-full" disabled={pending}>
        {pending ? 'Signing in…' : 'Sign in with username'}
      </Button>
      <p className="text-center text-sm text-muted-foreground">
        Don&apos;t have an account?{' '}
        <Link href="/signup" className="font-medium text-foreground underline">
          Sign up
        </Link>
      </p>
    </form>
  );
}
