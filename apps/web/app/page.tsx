import { Playground } from '../components/playground/Playground';
import { getServerUrl } from '../lib/server-url';

export default function Home() {
  const serverUrl = getServerUrl();

  return (
    <main className="mx-auto max-w-6xl px-6 py-12">
      <header className="mb-10">
        <p className="font-mono text-xs uppercase tracking-[0.2em] text-[color:var(--color-text-muted)]">
          TrustLoopGuard
        </p>
        <h1 className="mt-2 text-4xl font-semibold tracking-tight">Playground</h1>
        <p className="mt-3 max-w-2xl text-[color:var(--color-text-muted)]">
          Compose a CheckRequest, send it to tl-server, and inspect the Decision. Inputs are
          validated client-side with zod; the response is re-parsed at the boundary so the SDK
          consumer stays typesafe end-to-end.
        </p>
      </header>

      <Playground />

      <footer className="mt-12 flex items-center justify-between border-t border-[color:var(--color-border)] pt-4 font-mono text-xs text-[color:var(--color-text-muted)]">
        <span>POST {serverUrl}/v1/check</span>
        <span>override via NEXT_PUBLIC_TL_SERVER_URL</span>
      </footer>
    </main>
  );
}
