'use client';

import { useState, useTransition } from 'react';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { env } from '@/env';
import { sha256Hex } from '@/lib/password';

const MIN_PASSWORD_LEN = 8;
const MAX_PASSWORD_LEN = 128;

interface ChangePasswordCardProps {
  username: string;
}

export function ChangePasswordCard({ username }: ChangePasswordCardProps) {
  const [current, setCurrent] = useState('');
  const [next, setNext] = useState('');
  const [confirm, setConfirm] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setSuccess(null);

    if (next.length < MIN_PASSWORD_LEN || next.length > MAX_PASSWORD_LEN) {
      setError(`New password must be ${MIN_PASSWORD_LEN}-${MAX_PASSWORD_LEN} characters`);
      return;
    }
    if (next === current) {
      setError('New password must differ from current password');
      return;
    }
    if (next !== confirm) {
      setError('New password and confirmation do not match');
      return;
    }

    startTransition(async () => {
      const [currentHashed, nextHashed] = await Promise.all([sha256Hex(current), sha256Hex(next)]);
      const res = await fetch(`${env.NEXT_PUBLIC_TL_SERVER_URL}/v1/auth/password`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          username,
          current_password: currentHashed,
          new_password: nextHashed,
        }),
      });
      if (!res.ok) {
        const body = (await res.json().catch(() => null)) as { message?: string } | null;
        setError(body?.message ?? 'Could not update password');
        return;
      }
      setSuccess('Password updated');
      setCurrent('');
      setNext('');
      setConfirm('');
    });
  }

  return (
    <Card>
      <CardHeader>
        <CardDescription>Security</CardDescription>
        <CardTitle>Change password</CardTitle>
      </CardHeader>
      <CardContent>
        <form onSubmit={onSubmit} className="grid gap-4">
          <div className="grid gap-1.5">
            <Label htmlFor="current-password">Current password</Label>
            <Input
              id="current-password"
              type="password"
              autoComplete="current-password"
              value={current}
              onChange={(event) => setCurrent(event.target.value)}
              disabled={pending}
              required
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="new-password">New password</Label>
            <Input
              id="new-password"
              type="password"
              autoComplete="new-password"
              value={next}
              onChange={(event) => setNext(event.target.value)}
              disabled={pending}
              required
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="confirm-password">Confirm new password</Label>
            <Input
              id="confirm-password"
              type="password"
              autoComplete="new-password"
              value={confirm}
              onChange={(event) => setConfirm(event.target.value)}
              disabled={pending}
              required
            />
          </div>
          {error ? <p className="text-sm text-destructive">{error}</p> : null}
          {success ? <p className="text-sm text-emerald-600 dark:text-emerald-400">{success}</p> : null}
          <Button type="submit" disabled={pending} className="justify-self-start">
            {pending ? 'Updating…' : 'Update password'}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
