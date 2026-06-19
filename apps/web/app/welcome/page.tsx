import { redirect } from 'next/navigation';
import Link from 'next/link';
import { IconRefresh, IconUserCheck } from '@tabler/icons-react';

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
import { Separator } from '@/components/ui/separator';
import { CreateWorkspaceCard } from '@/components/workspace/CreateWorkspaceCard';
import { isWorkspaceSelfServiceEnabled } from '@/env';
import { getMyWorkspaces } from '@/lib/server/dashboard-data';

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
  const workspaces = await getMyWorkspaces({
    id: sessionUser.id,
    name: sessionUser.name ?? '',
    email,
    image: sessionUser.image ?? '',
    tlJwt: tlJwt !== undefined && tlJwt !== '' ? tlJwt : undefined,
  });
  if (workspaces.length > 0) {
    redirect(`/?workspace=${encodeURIComponent(workspaces[0]!.slug)}`);
  }

  const displayEmail = email !== '' ? email : (sessionUser.name ?? 'your account');
  const workspaceSelfServiceEnabled = isWorkspaceSelfServiceEnabled();
  const pageTitle = workspaceSelfServiceEnabled
    ? "You're not in a workspace yet"
    : 'Contact an admin to get access';
  const cardTitle = workspaceSelfServiceEnabled ? 'Waiting on an invite' : 'Access pending';

  async function signOutAction() {
    'use server';
    await signOut({ redirectTo: '/signin' });
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex min-h-screen w-full max-w-xl flex-col gap-8 px-4 py-10">
        <WelcomeBrandHeader status={workspaceSelfServiceEnabled ? 'Pending' : 'Access pending'} />

        <div className="flex flex-1 flex-col justify-center gap-6">
          <div className="grid gap-2">
            <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
              Account status
            </p>
            <h1 className="text-balance text-2xl font-semibold tracking-tight sm:text-3xl">
              {pageTitle}
            </h1>
          </div>

          <Card>
            <CardHeader>
              <CardTitle>{cardTitle}</CardTitle>
              {workspaceSelfServiceEnabled ? (
                <CardDescription>
                  An admin needs to add{' '}
                  <strong className="font-mono text-foreground">{displayEmail}</strong> to a
                  workspace before you can use the dashboard.
                </CardDescription>
              ) : (
                <CardDescription>
                  Please contact an admin to get access for{' '}
                  <strong className="font-mono text-foreground">{displayEmail}</strong>. They
                  need to approve your account and add you to a workspace before you can use
                  the dashboard.
                </CardDescription>
              )}
            </CardHeader>
            <CardContent className="grid gap-4">
              {workspaceSelfServiceEnabled ? (
                <ol className="grid gap-3">
                  {[
                    <>
                      Share{' '}
                      <strong className="font-mono text-foreground">{displayEmail}</strong> with
                      your team&apos;s admin.
                    </>,
                    <>
                      They invite you from their <strong>Team</strong> page.
                    </>,
                    <>Refresh this page once the invite is in place.</>,
                  ].map((step, index) => (
                    <li key={index} className="grid grid-cols-[auto_minmax(0,1fr)] items-start gap-3">
                      <span className="flex size-6 shrink-0 items-center justify-center rounded-md border border-border bg-muted font-mono text-xs tabular-nums text-muted-foreground">
                        {index + 1}
                      </span>
                      <span className="pt-0.5 text-sm leading-6 text-muted-foreground">
                        {step}
                      </span>
                    </li>
                  ))}
                </ol>
              ) : (
                <Alert>
                  <IconUserCheck />
                  <AlertTitle>Admin approval required</AlertTitle>
                  <AlertDescription>
                    Contact an admin with{' '}
                    <strong className="font-mono text-foreground">{displayEmail}</strong>. They
                    need to approve your account and add you to a workspace, then you can
                    refresh this page.
                  </AlertDescription>
                </Alert>
              )}
            </CardContent>
            <Separator />
            <CardFooter className="gap-2 pt-6">
              <Button asChild>
                <Link href="/welcome">
                  <IconRefresh />
                  Refresh
                </Link>
              </Button>
              <form action={signOutAction}>
                <Button type="submit" variant="ghost">
                  Sign out
                </Button>
              </form>
            </CardFooter>
          </Card>

          {workspaceSelfServiceEnabled ? <CreateWorkspaceCard /> : null}
        </div>
      </div>
    </main>
  );
}
