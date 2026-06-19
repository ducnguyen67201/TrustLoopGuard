'use client';

import { IconArrowRight, IconLoader2 } from '@tabler/icons-react';
import { useFormStatus } from 'react-dom';

import { Button } from '@/components/ui/button';

/**
 * Submit control for the create-workspace form. Lives inside the <form> so
 * useFormStatus() can read its pending state and show a disabled spinner while
 * the server action runs — no frozen screen on a slow network.
 */
export function SetupSubmitButton() {
  const { pending } = useFormStatus();
  return (
    <Button type="submit" size="lg" disabled={pending} aria-disabled={pending}>
      {pending ? (
        <>
          <IconLoader2 className="animate-spin motion-reduce:animate-none" aria-hidden />
          Creating workspace…
        </>
      ) : (
        <>
          Create workspace
          <IconArrowRight aria-hidden />
        </>
      )}
    </Button>
  );
}
