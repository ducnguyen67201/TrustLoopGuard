import type { Channel, Verdict } from '@trustloopguard/sdk';
import { getServerUrl } from '../lib/server-url';

// Smoke-test page. Imports types from the workspace SDK to prove the
// workspace:* link compiles end-to-end. PR 4 replaces this with the
// actual playground form.

export default function Home() {
  const serverUrl = getServerUrl();

  const channels: Channel[] = ['voice', 'chat', 'email'];
  const verdicts: Verdict[] = ['allow', 'block', 'rewrite', 'escalate'];

  return (
    <main className="mx-auto max-w-3xl px-6 py-16">
      <header className="mb-10">
        <p className="font-mono text-xs uppercase tracking-[0.2em] text-[color:var(--color-text-muted)]">
          TrustLoopGuard
        </p>
        <h1 className="mt-2 text-4xl font-semibold tracking-tight">Playground</h1>
        <p className="mt-3 max-w-xl text-[color:var(--color-text-muted)]">
          Workspace scaffold landed. The interactive form lands in the next PR. This page exists to
          verify the SDK type imports compile end-to-end through the workspace.
        </p>
      </header>

      <section className="rounded-lg border border-[color:var(--color-border)] bg-[color:var(--color-surface)] p-6">
        <h2 className="font-mono text-sm uppercase tracking-wider text-[color:var(--color-text-muted)]">
          Wiring check
        </h2>
        <dl className="mt-4 space-y-3 text-sm">
          <div className="flex justify-between gap-4">
            <dt className="text-[color:var(--color-text-muted)]">Server URL</dt>
            <dd className="font-mono">{serverUrl}</dd>
          </div>
          <div className="flex justify-between gap-4">
            <dt className="text-[color:var(--color-text-muted)]">SDK channels</dt>
            <dd className="font-mono">{channels.join(', ')}</dd>
          </div>
          <div className="flex justify-between gap-4">
            <dt className="text-[color:var(--color-text-muted)]">SDK verdicts</dt>
            <dd className="font-mono">{verdicts.join(', ')}</dd>
          </div>
        </dl>
      </section>
    </main>
  );
}
