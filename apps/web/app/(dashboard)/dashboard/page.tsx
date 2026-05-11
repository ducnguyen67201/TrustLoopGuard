import Link from 'next/link';

import { auth } from '@/auth';

export default async function DashboardPage() {
  const session = await auth();

  return (
    <main className="mx-auto max-w-4xl px-6 py-12">
      <h1 className="text-3xl font-semibold tracking-tight">Dashboard</h1>
      <p className="mt-3 text-sm text-[color:var(--color-text-muted)]">
        Signed in as {session?.user?.email ?? 'unknown'}.
      </p>
      <nav className="mt-8">
        <Link
          href="/dashboard/keys"
          className="inline-flex rounded border border-[color:var(--color-border)] px-4 py-2 text-sm hover:bg-[color:var(--color-surface-elevated)]"
        >
          Manage API keys
        </Link>
      </nav>
    </main>
  );
}
