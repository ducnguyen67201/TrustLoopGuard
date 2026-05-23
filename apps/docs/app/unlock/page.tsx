import { safeDocsRedirectPath } from '@/lib/docs-auth';

type UnlockPageProps = {
  searchParams?: Promise<{
    error?: string;
    next?: string;
  }>;
};

export default async function UnlockPage({ searchParams }: UnlockPageProps) {
  const params = await searchParams;
  const nextPath = safeDocsRedirectPath(params?.next);
  const hasError = params?.error === '1';

  return (
    <main className="flex min-h-screen items-center justify-center bg-fd-background px-6 py-12">
      <form
        action="/api/docs-auth"
        method="post"
        className="w-full max-w-sm space-y-5 rounded-lg border bg-fd-card p-6 shadow-sm"
      >
        <input type="hidden" name="next" value={nextPath} />

        <div className="space-y-2">
          <h1 className="text-xl font-semibold tracking-normal text-fd-foreground">
            TrustLoopGuard docs
          </h1>
          <p className="text-sm text-fd-muted-foreground">
            Enter the docs password to continue.
          </p>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium text-fd-foreground" htmlFor="password">
            Password
          </label>
          <input
            id="password"
            name="password"
            type="password"
            autoComplete="current-password"
            autoFocus
            required
            className="w-full rounded-md border bg-fd-background px-3 py-2 text-sm outline-none ring-fd-ring transition focus:ring-2"
          />
          {hasError ? (
            <p className="text-sm font-medium text-red-600">That password did not work.</p>
          ) : null}
        </div>

        <button
          type="submit"
          className="w-full rounded-md bg-fd-primary px-4 py-2 text-sm font-medium text-fd-primary-foreground transition hover:opacity-90"
        >
          Unlock docs
        </button>
      </form>
    </main>
  );
}
