import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

import { CredentialsForm } from './credentials-form';

export default async function SignInPage({
  searchParams,
}: {
  searchParams: Promise<{ callbackUrl?: string | string[] }>;
}) {
  const params = await searchParams;
  const callbackUrl = safeRedirect(
    Array.isArray(params.callbackUrl) ? params.callbackUrl[0] : params.callbackUrl,
  );

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10 text-foreground">
      <div className="grid w-full max-w-xl gap-4">
        <div className="grid gap-2">
          <div className="text-sm text-muted-foreground">TrustLoopGuard</div>
          <h1 className="text-2xl font-semibold">Sign in</h1>
        </div>
        <Card>
          <CardHeader>
            <CardTitle>Sign in to TrustLoopGuard</CardTitle>
            <CardDescription>Sign in with your TrustLoopGuard username.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <CredentialsForm callbackUrl={callbackUrl} />
          </CardContent>
        </Card>
      </div>
    </main>
  );
}

function safeRedirect(value: FormDataEntryValue | string | undefined | null): string {
  if (typeof value !== 'string' || value.trim() === '') return '/';
  if (!value.startsWith('/') || value.startsWith('//')) return '/';
  if (value.startsWith('/signin') || value.startsWith('/api/auth')) return '/';
  return value;
}
