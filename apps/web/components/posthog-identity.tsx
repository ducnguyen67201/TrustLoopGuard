'use client';

import { useEffect } from 'react';
import posthog from 'posthog-js';

import { identifyPostHogUser, type PostHogUser } from '@/lib/posthog';

export function PostHogIdentity({ user }: { user: PostHogUser }) {
  useEffect(() => {
    identifyPostHogUser(posthog, user);
  }, [user.email, user.id, user.name]);

  return null;
}
