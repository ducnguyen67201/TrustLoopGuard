import { auth, signOut } from '@/auth';
import { PostHogSignOutButton } from '@/components/posthog-sign-out-button';

/**
 * Shared frame for the pre-shell onboarding steps (workspace → connect →
 * verify). These pages render outside AppLayout, so without this a user who
 * signed in with the wrong account had no way out mid-onboarding — the
 * sign-out affordance must always be reachable.
 */
export default async function OnboardingLayout({ children }: { children: React.ReactNode }) {
  const session = await auth();
  const email = session?.user?.email?.trim() ?? '';

  async function signOutAction() {
    'use server';
    await signOut({ redirectTo: '/signin' });
  }

  return (
    <div className="relative">
      <div className="absolute right-4 top-4 z-10 flex items-center gap-2">
        {email !== '' ? (
          <span className="hidden text-xs text-muted-foreground sm:inline">{email}</span>
        ) : null}
        <form action={signOutAction}>
          <PostHogSignOutButton variant="ghost" size="sm">
            Sign out
          </PostHogSignOutButton>
        </form>
      </div>
      {children}
    </div>
  );
}
