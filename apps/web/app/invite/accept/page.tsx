import { redirect } from 'next/navigation';
import Link from 'next/link';

import { SignupForm } from '@/app/(auth)/signup/signup-form';
import { auth } from '@/auth';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { getServerUrl } from '@/lib/server-url';

interface InviteLookup {
  email: string;
  role: string;
  workspace_name: string;
  workspace_slug: string;
  status: 'pending' | 'accepted' | 'revoked' | 'expired';
  expires_at: string;
  user_exists: boolean;
}

export default async function InviteAcceptPage({
  searchParams,
}: {
  searchParams: Promise<{ token?: string | string[] }>;
}) {
  const params = await searchParams;
  const tokenRaw = Array.isArray(params.token) ? params.token[0] : params.token;
  const token = tokenRaw?.trim();

  if (token === undefined || token === '') {
    return (
      <Shell title="Invite link missing">
        <p className="text-sm text-muted-foreground">
          This link doesn&apos;t contain an invite token. Ask your admin for a
          fresh invite.
        </p>
      </Shell>
    );
  }

  const lookup = await fetchLookup(token);
  if (lookup === null) {
    return (
      <Shell title="Invite not found">
        <p className="text-sm text-muted-foreground">
          We couldn&apos;t look up this invite. It may have been revoked or it
          may have expired.
        </p>
      </Shell>
    );
  }

  if (lookup.status === 'revoked') {
    return (
      <Shell title="Invite revoked" workspace={lookup.workspace_name}>
        <p className="text-sm text-muted-foreground">
          This invite was revoked. Ask your admin to send a new one.
        </p>
      </Shell>
    );
  }

  if (lookup.status === 'expired') {
    return (
      <Shell title="Invite expired" workspace={lookup.workspace_name}>
        <p className="text-sm text-muted-foreground">
          This invite expired. Ask your admin to send a new one.
        </p>
      </Shell>
    );
  }

  if (lookup.status === 'accepted') {
    return (
      <Shell title="Already accepted" workspace={lookup.workspace_name}>
        <p className="text-sm text-muted-foreground">
          This invite has already been accepted.{' '}
          <Link href="/signin" className="font-medium text-foreground underline">
            Sign in
          </Link>{' '}
          to continue.
        </p>
      </Shell>
    );
  }

  // Already signed in → /welcome will auto-bind the pending invite
  // on its next /v1/team/my-workspaces call and bounce to the
  // workspace immediately. Skip the accept-page UI entirely.
  const session = await auth();
  if (session?.user?.id !== undefined && session.user.id !== '') {
    redirect('/welcome');
  }

  // Account exists but not signed in → /signin, then /welcome,
  // then auto-bind. No accept-page UI needed.
  if (lookup.user_exists) {
    redirect(`/signin?callbackUrl=${encodeURIComponent('/welcome')}`);
  }

  const callbackUrl = `/?workspace=${encodeURIComponent(lookup.workspace_slug)}`;

  return (
    <Shell
      title={`Join ${lookup.workspace_name}`}
      workspace={lookup.workspace_name}
    >
      <p className="text-sm text-muted-foreground">
        You&apos;ve been invited as <strong>{lookup.role}</strong>. Create an
        account for <strong>{lookup.email}</strong> to accept.
      </p>
      <SignupForm
        callbackUrl={callbackUrl}
        inviteToken={token}
        presetUsername={lookup.email}
      />
    </Shell>
  );
}

function Shell({
  title,
  workspace,
  children,
}: {
  title: string;
  workspace?: string;
  children: React.ReactNode;
}) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10 text-foreground">
      <div className="grid w-full max-w-xl gap-4">
        <div className="grid gap-2">
          <div className="text-sm text-muted-foreground">TrustLoopGuard</div>
          <h1 className="text-2xl font-semibold">{title}</h1>
        </div>
        <Card>
          <CardHeader>
            <CardTitle>{workspace ?? 'Workspace invitation'}</CardTitle>
            <CardDescription>
              Invites are single-use and expire after 7 days.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4">{children}</CardContent>
        </Card>
      </div>
    </main>
  );
}

async function fetchLookup(token: string): Promise<InviteLookup | null> {
  try {
    const res = await fetch(
      `${getServerUrl()}/v1/invites/${encodeURIComponent(token)}/lookup`,
      { cache: 'no-store' },
    );
    if (!res.ok) return null;
    return (await res.json()) as InviteLookup;
  } catch {
    return null;
  }
}
