'use client';

import { useState } from 'react';
import { signIn } from 'next-auth/react';
import { IconBrandGithub, IconBrandGoogle } from '@tabler/icons-react';

import { Button } from '@/components/ui/button';

interface OAuthButtonsProps {
  callbackUrl: string;
  providers: {
    github: boolean;
    google: boolean;
  };
}

export function OAuthButtons({ callbackUrl, providers }: OAuthButtonsProps) {
  const [pendingProvider, setPendingProvider] = useState<'github' | 'google' | null>(null);
  const hasProvider = providers.google || providers.github;

  if (!hasProvider) return null;

  async function authenticate(
    event: React.MouseEvent<HTMLButtonElement>,
    provider: 'github' | 'google',
  ) {
    event.preventDefault();
    event.stopPropagation();
    setPendingProvider(provider);
    await signIn(provider, { callbackUrl });
  }

  return (
    <div className="grid gap-2">
      {providers.google ? (
        <Button
          type="button"
          className="w-full"
          disabled={pendingProvider === 'google'}
          onClick={(event) => authenticate(event, 'google')}
        >
          <IconBrandGoogle className="size-4" aria-hidden="true" />
          Continue with Google
        </Button>
      ) : null}
      {providers.github ? (
        <Button
          type="button"
          className="w-full"
          disabled={pendingProvider === 'github'}
          onClick={(event) => authenticate(event, 'github')}
        >
          <IconBrandGithub className="size-4" aria-hidden="true" />
          Continue with GitHub
        </Button>
      ) : null}
    </div>
  );
}
