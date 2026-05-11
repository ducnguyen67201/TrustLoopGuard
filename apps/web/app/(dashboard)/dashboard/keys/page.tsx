import { redirect } from 'next/navigation';

import { auth } from '@/auth';
import { env } from '@/env';
import { AdminApiError, listKeys, type ApiKeyView } from '@/lib/tl-admin/client';

import { KeysClient } from './keys-client';

export default async function KeysPage() {
  const session = await auth();
  if (!session?.user?.id) {
    redirect('/signin');
  }

  if (!env.TL_ADMIN_KEY) {
    return (
      <main className="mx-auto max-w-4xl px-6 py-12">
        <h1 className="text-3xl font-semibold tracking-tight">API keys</h1>
        <p className="mt-3 text-sm text-[color:var(--color-text-muted)]">
          API key management is not configured on this deployment. Set
          <code className="mx-1 font-mono">TL_ADMIN_KEY</code>
          on the web server to enable it.
        </p>
      </main>
    );
  }

  let keys: ApiKeyView[];
  let loadError: string | null = null;
  try {
    keys = await listKeys(session.user.id);
  } catch (err) {
    keys = [];
    loadError =
      err instanceof AdminApiError
        ? err.message
        : 'Could not reach the guard server.';
  }

  return (
    <main className="mx-auto max-w-4xl px-6 py-12">
      <header className="flex items-baseline justify-between">
        <h1 className="text-3xl font-semibold tracking-tight">API keys</h1>
        <p className="text-sm text-[color:var(--color-text-muted)]">
          Signed in as {session.user.email}
        </p>
      </header>
      <p className="mt-3 text-sm text-[color:var(--color-text-muted)]">
        Use these tokens as the bearer for <code className="font-mono">/v1/check</code>.
        The plaintext is shown once when you create a key — store it somewhere safe.
      </p>

      {loadError ? (
        <p className="mt-6 rounded border border-red-500/40 bg-red-500/10 p-3 text-sm text-red-700" role="alert">
          {loadError}
        </p>
      ) : null}

      <KeysClient initialKeys={keys} />
    </main>
  );
}
