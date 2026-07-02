import { OnboardingProgress } from '@/components/onboarding/OnboardingProgress';
import { VerifyStep } from '@/components/onboarding/VerifyStep';
import { readParam } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';

// Onboarding step 3 of 3. getDashboardShell handles the guards (signin /
// zero-workspace bounce); the client panel finds an already-existing first
// trace on its immediate first poll.
export default async function VerifyOnboardingPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const environmentId = readParam(params.environment);
  const shell = await getDashboardShell(workspaceSlug, environmentId);

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex w-full max-w-xl flex-col gap-8 px-4 py-10">
        <OnboardingProgress current={3} />

        <div className="grid gap-3">
          <h1 className="text-balance text-2xl font-semibold tracking-tight sm:text-3xl">
            Waiting for your first decision
          </h1>
          <p className="max-w-prose text-sm leading-6 text-muted-foreground">
            Every request your agent makes gets checked and shows up here — allowed, cleaned up,
            held for review, or blocked. This page updates itself.
          </p>
        </div>

        <VerifyStep workspaceSlug={shell.activeWorkspace.slug} />
      </div>
    </main>
  );
}
