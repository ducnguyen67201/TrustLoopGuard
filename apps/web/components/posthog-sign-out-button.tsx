'use client';

import type { ComponentProps } from 'react';
import posthog from 'posthog-js';

import { Button } from '@/components/ui/button';
import { resetPostHogIdentity } from '@/lib/posthog';

export function PostHogSignOutButton({ onClick, ...props }: ComponentProps<typeof Button>) {
  return (
    <Button
      {...props}
      type="submit"
      onClick={(event) => {
        resetPostHogIdentity(posthog);
        onClick?.(event);
      }}
    />
  );
}
