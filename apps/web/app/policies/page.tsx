import Link from 'next/link';
import { PolicyManager } from '@/components/policies/PolicyManager';
import { getServerUrl } from '@/lib/server-url';

export default function PoliciesPage() {
  const serverUrl = getServerUrl();

  return (
    <main className="mx-auto max-w-7xl px-6 py-8">
      <header className="mb-8 flex flex-col gap-4 border-b border-border pb-6 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="font-mono text-xs uppercase tracking-[0.2em] text-muted-foreground">
            TrustLoopGuard
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-tight">Policy Manager</h1>
        </div>
        <nav className="flex items-center gap-2 font-mono text-sm">
          <Link
            href="/"
            className="rounded-md border border-border px-3 py-2 text-muted-foreground transition hover:bg-muted hover:text-foreground"
          >
            Playground
          </Link>
          <Link
            href="/policies"
            className="rounded-md bg-primary px-3 py-2 text-primary-foreground"
          >
            Policies
          </Link>
        </nav>
      </header>

      <PolicyManager />

      <footer className="mt-8 flex items-center justify-between border-t border-border pt-4 font-mono text-xs text-muted-foreground">
        <span>{serverUrl}/v1/policies</span>
        <span>YAML + JSON API</span>
      </footer>
    </main>
  );
}
