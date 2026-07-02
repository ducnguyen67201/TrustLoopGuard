import { ConnectAgentStep } from '@/components/onboarding/ConnectAgentStep';
import { OnboardingProgress } from '@/components/onboarding/OnboardingProgress';
import { env } from '@/env';
import { readParam } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';

// Onboarding step 2 of 3. getDashboardShell handles the guards: unauthenticated
// users bounce to /signin, users with zero workspaces bounce back to step 1.
export default async function ConnectOnboardingPage({
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
        <OnboardingProgress current={2} />

        <div className="grid gap-3">
          <h1 className="text-balance text-2xl font-semibold tracking-tight sm:text-3xl">
            Connect your agent
          </h1>
          <p className="max-w-prose text-sm leading-6 text-muted-foreground">
            Create an API key, then wire the guard into your app — by hand with the SDK, or by
            handing the prompt below to your AI coding assistant. Two minutes either way.
          </p>
        </div>

        <ConnectAgentStep
          baseUrl={env.NEXT_PUBLIC_TL_SERVER_URL}
          environmentId={shell.activeEnvironment.id}
          defaultAgentId={shell.agents[0]?.id ?? 'my-agent'}
          workspaceSlug={shell.activeWorkspace.slug}
          requestedEnvironmentId={environmentId ?? undefined}
        />
      </div>
    </main>
  );
}
