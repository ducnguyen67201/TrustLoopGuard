import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { BrandLogo } from '@/components/brand-logo';
import { Separator } from '@/components/ui/separator';
import { getAuthCapabilities, hasOAuthProvider } from '@/lib/auth-capabilities';

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
  const authCapabilities = getAuthCapabilities();
  const oauthConfigured = hasOAuthProvider(authCapabilities.oauthProviders);

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10 text-foreground">
      <div className="grid w-full max-w-xl gap-4">
        <div className="grid gap-2">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <BrandLogo className="size-7" priority />
            <span>TrustLoopGuard</span>
          </div>
          <h1 className="text-2xl font-semibold">Sign in</h1>
        </div>
        <Card>
          <CardHeader>
            <CardTitle>Sign in to TrustLoopGuard</CardTitle>
            <CardDescription>
              {authCapabilities.credentials
                ? 'Use your workspace identity or TrustLoopGuard username.'
                : 'Use your workspace identity to continue.'}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {oauthConfigured ? (
              <OAuthButtons callbackUrl={callbackUrl} providers={authCapabilities.oauthProviders} />
            ) : null}
            {oauthConfigured && authCapabilities.credentials ? (
              <div className="flex items-center gap-3">
                <Separator className="flex-1" />
                <span className="text-xs text-muted-foreground">or</span>
                <Separator className="flex-1" />
              </div>
            ) : null}
            {authCapabilities.credentials ? <CredentialsForm callbackUrl={callbackUrl} /> : null}
            {!oauthConfigured && !authCapabilities.credentials ? (
              <p className="text-sm text-muted-foreground">
                No OAuth provider is configured for this deployment.
              </p>
            ) : null}
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
