import Link from 'next/link';
import { Playground } from '../components/playground/Playground';
import { getServerUrl } from '../lib/server-url';

export default function Home() {
  const serverUrl = getServerUrl();

  return (
    <main className="mx-auto max-w-6xl px-6 py-12">
      <header className="mb-10 flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="font-mono text-xs uppercase tracking-[0.2em] text-muted-foreground">
            TrustLoopGuard
          </p>
          <h1 className="mt-2 text-4xl font-semibold tracking-tight">Playground</h1>
        </div>
        <nav className="flex items-center gap-2 font-mono text-sm">
          <Link href="/" className="rounded-md bg-primary px-3 py-2 text-primary-foreground">
            Playground
          </Link>
          <Link
            href="/policies"
            className="rounded-md border border-border px-3 py-2 text-muted-foreground transition hover:bg-muted hover:text-foreground"
          >
            Policies
          </Link>
        </nav>
      </header>

      <Playground />

      <footer className="mt-12 flex items-center justify-between border-t border-[color:var(--color-border)] pt-4 font-mono text-xs text-[color:var(--color-text-muted)]">
        <span>POST {serverUrl}/v1/check</span>
        <span>override via NEXT_PUBLIC_TL_SERVER_URL</span>
      </footer>
    </main>
  );
}
