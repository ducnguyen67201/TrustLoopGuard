import { redirect } from 'next/navigation';

import { auth } from '@/auth';
import { rustApiForUser } from '@/lib/server/tl-client';
import type { WorkspaceMembership } from '@/lib/workspace-access';

import { ConsentForm } from './consent-form';

export const runtime = 'nodejs';

// OAuth 2.1 authorize endpoint for MCP clients. Reuses the existing Auth.js
// login; on consent it calls the Next route which asks tl-server to mint a
// PKCE-bound authorization code scoped to the chosen workspace.
export default async function OAuthAuthorizePage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const params = await searchParams;
  const get = (key: string): string | undefined => {
    const value = params[key];
    return Array.isArray(value) ? value[0] : value;
  };

  const session = await auth();
  const user = session?.user as { id?: string; email?: string | null } | undefined;
  const userId = user?.id;

  if (!userId) {
    // Bounce to the platform login, preserving the OAuth params so we return
    // here after authentication.
    const qs = new URLSearchParams();
    for (const key of [
      'client_id',
      'redirect_uri',
      'state',
      'code_challenge',
      'code_challenge_method',
      'scope',
    ]) {
      const value = get(key);
      if (value) qs.set(key, value);
    }
    redirect(`/signin?callbackUrl=${encodeURIComponent(`/oauth/authorize?${qs.toString()}`)}`);
  }

  const clientId = get('client_id');
  const redirectUri = get('redirect_uri');
  const state = get('state');
  const codeChallenge = get('code_challenge');

  if (!clientId || !redirectUri || !state || !codeChallenge) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background p-6">
        <p className="text-sm text-muted-foreground">
          Invalid authorization request — missing client_id, redirect_uri, state, or code_challenge.
        </p>
      </main>
    );
  }

  const { workspaces } = await rustApiForUser<{ workspaces: WorkspaceMembership[] }>(
    { id: userId, email: user?.email ?? undefined },
    '/v1/team/my-workspaces',
  );

  if (workspaces.length === 0) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background p-6">
        <p className="text-sm text-muted-foreground">
          You don&apos;t belong to any workspace yet. Create or join one first.
        </p>
      </main>
    );
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-6">
      <div className="w-full max-w-md space-y-6 rounded-lg border border-border bg-card p-6">
        <div className="space-y-1">
          <h1 className="text-xl font-semibold text-foreground">Authorize access</h1>
          <p className="text-sm text-muted-foreground">
            An MCP client wants to act on your TrustLoopGuard workspace. Choose which workspace to
            grant access to.
          </p>
        </div>
        <ConsentForm
          clientId={clientId}
          redirectUri={redirectUri}
          state={state}
          codeChallenge={codeChallenge}
          workspaces={workspaces}
          userLabel={user?.email ?? userId}
        />
      </div>
    </main>
  );
}
