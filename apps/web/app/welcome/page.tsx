import { redirect } from 'next/navigation';
import Link from 'next/link';

import { auth, signOut } from '@/auth';
import { BrandLogo } from '@/components/brand-logo';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { CreateWorkspaceCard } from '@/components/workspace/CreateWorkspaceCard';
import { getMyWorkspaces } from '@/lib/server/dashboard-data';

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

  async function signOutAction() {
    'use server';
    await signOut({ redirectTo: '/signin' });
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10 text-foreground">
      <div className="grid w-full max-w-xl gap-4">
        <div className="grid gap-2">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <BrandLogo className="size-7" priority />
            <span>TrustLoopGuard</span>
          </div>
          <h1 className="text-2xl font-semibold">You&apos;re not in a workspace yet</h1>
        </div>
        <Card>
          <CardHeader>
            <CardTitle>Waiting on an invite</CardTitle>
            <CardDescription>
              An admin needs to add <strong>{displayEmail}</strong> to a workspace{' '}
              before you can use the dashboard.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4">
            <ol className="ml-4 list-decimal space-y-1 text-sm text-muted-foreground">
              <li>
                Share <strong>{displayEmail}</strong>{' '}with your team&apos;s admin.
              </li>
              <li>
                They invite you from their <strong>Team</strong> page.
              </li>
              <li>
                Refresh this page — you&apos;ll be redirected automatically the
                moment the invite is in place.
              </li>
            </ol>
            <div className="flex flex-wrap gap-2">
              <Button asChild>
                <Link href="/welcome">Refresh</Link>
              </Button>
              <form action={signOutAction}>
                <Button type="submit" variant="ghost">
                  Sign out
                </Button>
              </form>
            </div>
          </CardContent>
        </Card>
        <CreateWorkspaceCard />
      </div>
    </main>
  );
}
