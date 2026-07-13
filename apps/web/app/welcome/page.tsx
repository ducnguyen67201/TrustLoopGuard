import { redirect } from 'next/navigation';
import Link from 'next/link';
import { IconMail, IconRefresh, IconUserCheck } from '@tabler/icons-react';

import { auth, signOut } from '@/auth';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { InfoHint } from '@/components/ui/info-hint';
import { PostHogSignOutButton } from '@/components/posthog-sign-out-button';
import { Separator } from '@/components/ui/separator';
import { CreateWorkspaceCard } from '@/components/workspace/CreateWorkspaceCard';
import { approvedWorkspaceLandingPath } from '@/lib/onboarding';
import { getDashboardShell, getWorkspaceAccessState } from '@/lib/server/dashboard-data';

import { WelcomeBrandHeader } from './WelcomeBrandHeader';

export default async function WelcomePage() {
  const session = await auth();
  const sessionUser = session?.user;
  if (sessionUser?.id === undefined || sessionUser.id === '') {
    redirect('/signin?callbackUrl=/welcome');
  }

  // Re-check on every visit so the page acts as a refresh point: if
  // the admin invited the user between visits, the auto-bind on the
  // Rust side will surface the new workspace here and we'll bounce
  // straight to it.
  const email = sessionUser.email?.trim() ?? '';
  const tlJwt = (sessionUser as { tlJwt?: string }).tlJwt;
  const access = await getWorkspaceAccessState({
    id: sessionUser.id,
    name: sessionUser.name ?? '',
    email,
    image: sessionUser.image ?? '',
    tlJwt: tlJwt !== undefined && tlJwt !== '' ? tlJwt : undefined,
  });
  const workspaces = access.kind === 'ready' ? access.workspaces : [];
  if (workspaces.length > 0) {
    const shell = await getDashboardShell(workspaces[0]!.slug);
    redirect(
      approvedWorkspaceLandingPath({
        workspaceSlug: shell.activeWorkspace.slug,
        agentCount: shell.activeWorkspace.agentCount,
        environmentId: shell.activeEnvironment.id,
      }),
    );
  }

  const displayEmail = email !== '' ? email : (sessionUser.name ?? 'your account');
  const approvalPending = access.kind === 'pending_approval';
  const pageTitle = approvalPending
    ? "You're signed in — waiting for approval"
    : "You're all set — just need a workspace";
  const cardTitle = approvalPending ? 'Waiting for admin approval' : 'Waiting for an invite';

  async function signOutAction() {
    'use server';
    await signOut({ redirectTo: '/signin' });
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex min-h-screen w-full max-w-xl flex-col gap-8 px-4 py-10">
        <WelcomeBrandHeader status={approvalPending ? 'Awaiting access' : 'Almost there'} />

        <div className="flex flex-1 flex-col justify-center gap-6">
          <div className="grid gap-3">
            <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
              Account status
            </p>
            <h1 className="text-balance text-2xl font-semibold tracking-tight sm:text-3xl">
              {pageTitle}
            </h1>
            <p className="max-w-prose text-sm leading-6 text-muted-foreground">
              {approvalPending ? (
                <>
                  Your account exists, but an admin needs to approve{' '}
                  <strong className="font-mono text-foreground">{displayEmail}</strong> before you
                  can create or join a workspace.
                </>
              ) : (
                <>
                  Your account is ready. To start, either join a teammate&apos;s{' '}
                  <span className="inline-flex items-center gap-1 font-medium text-foreground">
                    workspace
                    <InfoHint term="workspace" />
                  </span>{' '}
                  or create your own below — it only takes a moment.
                </>
              )}
            </p>
          </div>

          {!approvalPending ? <CreateWorkspaceCard /> : null}

          <Card>
            <CardHeader>
              <CardTitle>{cardTitle}</CardTitle>
              {approvalPending ? (
                <CardDescription>
                  Ask an admin to approve{' '}
                  <strong className="font-mono text-foreground">{displayEmail}</strong>. Once
                  approved, refresh and you&apos;ll continue setup.
                </CardDescription>
              ) : (
                <CardDescription>
                  Prefer to be added to an existing workspace? Ask a teammate to invite{' '}
                  <strong className="font-mono text-foreground">{displayEmail}</strong>, then
                  refresh — you&apos;ll be taken straight in.
                </CardDescription>
              )}
            </CardHeader>
            <CardContent className="grid gap-4">
              {approvalPending ? (
                <Alert>
                  <IconUserCheck />
                  <AlertTitle>What to do</AlertTitle>
                  <AlertDescription>
                    Send your email,{' '}
                    <strong className="font-mono text-foreground">{displayEmail}</strong>, to an
                    admin and ask them to approve your account. Then come back and hit{' '}
                    <strong>Refresh</strong>.
                  </AlertDescription>
                </Alert>
              ) : (
                <ol className="grid gap-3">
                  {[
                    <>
                      Send your email,{' '}
                      <strong className="font-mono text-foreground">{displayEmail}</strong>, to your
                      team&apos;s admin.
                    </>,
                    <>
                      They add you from their <strong>Team</strong> page — no action needed from
                      you.
                    </>,
                    <>
                      Come back and hit <strong>Refresh</strong>. We&apos;ll take you to the
                      dashboard automatically.
                    </>,
                  ].map((step, index) => (
                    <li
                      key={index}
                      className="grid grid-cols-[auto_minmax(0,1fr)] items-start gap-3"
                    >
                      <span className="flex size-6 shrink-0 items-center justify-center rounded-md border border-border bg-muted font-mono text-xs tabular-nums text-muted-foreground">
                        {index + 1}
                      </span>
                      <span className="pt-0.5 text-sm leading-6 text-muted-foreground">{step}</span>
                    </li>
                  ))}
                </ol>
              )}
              <p className="flex items-start gap-2 rounded-lg border border-border bg-muted/40 p-3 text-xs leading-5 text-muted-foreground">
                <IconMail className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" aria-hidden />
                Nothing to install or set up while you wait. You can safely close this tab and come
                back later — you&apos;ll land right back here.
              </p>
            </CardContent>
            <Separator />
            <CardFooter className="gap-2 pt-6">
              <Button asChild>
                <Link href="/welcome">
                  <IconRefresh aria-hidden />
                  Check again
                </Link>
              </Button>
              <form action={signOutAction}>
                <PostHogSignOutButton variant="ghost">Sign out</PostHogSignOutButton>
              </form>
            </CardFooter>
          </Card>
        </div>
      </div>
    </main>
  );
}
