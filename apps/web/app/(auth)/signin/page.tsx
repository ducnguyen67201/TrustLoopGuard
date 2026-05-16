import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { env } from '@/env';

import { CredentialsForm } from './credentials-form';
import { OAuthButtons } from './oauth-buttons';

export default async function SignInPage({
  searchParams,
}: {
  searchParams: Promise<{ callbackUrl?: string | string[] }>;
}) {
  const params = await searchParams;
  const callbackUrl = safeRedirect(
    Array.isArray(params.callbackUrl) ? params.callbackUrl[0] : params.callbackUrl,
  );
  const oauthProviders = {
    github: Boolean(env.AUTH_GITHUB_ID && env.AUTH_GITHUB_SECRET),
    google: Boolean(env.AUTH_GOOGLE_ID && env.AUTH_GOOGLE_SECRET),
  };
  const hasOAuthProvider = oauthProviders.github || oauthProviders.google;

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
            <CardDescription>Use your workspace identity or TrustLoopGuard username.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <OAuthButtons callbackUrl={callbackUrl} providers={oauthProviders} />
            {hasOAuthProvider ? (
              <div className="flex items-center gap-3">
                <Separator className="flex-1" />
                <span className="text-xs text-muted-foreground">or</span>
                <Separator className="flex-1" />
              </div>
            ) : null}
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
