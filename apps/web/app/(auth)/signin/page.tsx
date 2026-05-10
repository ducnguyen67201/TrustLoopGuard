export default function SignInPage() {
  return (
    <main className="mx-auto max-w-md px-6 py-16">
      <h1 className="text-2xl font-semibold tracking-tight">Sign in</h1>
      <p className="mt-3 text-sm text-[color:var(--color-text-muted)]">
        No sign-in methods are configured for this deployment. Set
        <code className="mx-1 font-mono">AUTH_ALLOW_SIGNUP</code>
        to enable email and password, or
        <code className="mx-1 font-mono">AUTH_GOOGLE_ID</code>
        and
        <code className="mx-1 font-mono">AUTH_GOOGLE_SECRET</code>
        to enable Google.
      </p>
    </main>
  );
}
