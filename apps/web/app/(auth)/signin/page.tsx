import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { signIn } from '@/auth';
import { env } from '@/env';

import { CredentialsForm } from './credentials-form';

const providers = [
  {
    id: 'google',
    label: 'Continue with Google',
    enabled: Boolean(env.AUTH_GOOGLE_ID && env.AUTH_GOOGLE_SECRET),
  },
  {
    id: 'github',
    label: 'Continue with GitHub',
    enabled: Boolean(env.AUTH_GITHUB_ID && env.AUTH_GITHUB_SECRET),
  },
] as const;

async function signInWithProvider(formData: FormData) {
  'use server';

  const provider = formData.get('provider');
  const callbackUrl = formData.get('callbackUrl');
  if (provider !== 'google' && provider !== 'github') {
    return;
  }

  await signIn(provider, { redirectTo: safeRedirect(callbackUrl) });
}

export default async function SignInPage({
  searchParams,
}: {
  searchParams: Promise<{ callbackUrl?: string | string[] }>;
}) {
  const enabledProviders = providers.filter((provider) => provider.enabled);
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
            <CardDescription>
              Use Google or GitHub, or sign in with a self-hosted username.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {enabledProviders.length > 0 ? (
              <div className="space-y-3">
                {enabledProviders.map((provider) => (
                  <form key={provider.id} action={signInWithProvider}>
                    <input type="hidden" name="callbackUrl" value={callbackUrl} />
                    <Button className="w-full" type="submit" name="provider" value={provider.id}>
                      {provider.label}
                    </Button>
                  </form>
                ))}
              </div>
            ) : null}

            {enabledProviders.length > 0 ? (
              <div className="flex items-center gap-3">
                <Separator className="flex-1" />
                <span className="text-xs uppercase tracking-wide text-muted-foreground">or</span>
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
