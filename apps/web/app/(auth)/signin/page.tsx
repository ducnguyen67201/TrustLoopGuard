import { getAuthCapabilities, hasOAuthProvider } from '@/lib/auth-capabilities';

import { AuthScreen } from '../auth-screen';
import { OrDivider } from '../form-feedback';
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
    <AuthScreen
      eyebrow="Runtime guardrails"
      title="Welcome back to the control room."
      description="Sign in to inspect decisions, tune policies, and keep your AI agents inside the lines."
      cardTitle="Sign in"
      cardDescription={
        authCapabilities.credentials
          ? 'Use your workspace identity or TrustLoopGuard username.'
          : 'Use your workspace identity to continue.'
      }
    >
      {oauthConfigured ? (
        <OAuthButtons callbackUrl={callbackUrl} providers={authCapabilities.oauthProviders} />
      ) : null}
      {oauthConfigured && authCapabilities.credentials ? <OrDivider /> : null}
      {authCapabilities.credentials ? <CredentialsForm callbackUrl={callbackUrl} /> : null}
      {!oauthConfigured && !authCapabilities.credentials ? (
        <p className="text-sm text-muted-foreground">
          No OAuth provider is configured for this deployment.
        </p>
      ) : null}
    </AuthScreen>
  );
}

function safeRedirect(value: FormDataEntryValue | string | undefined | null): string {
  if (typeof value !== 'string' || value.trim() === '') return '/';
  if (!value.startsWith('/') || value.startsWith('//')) return '/';
  if (value.startsWith('/signin') || value.startsWith('/api/auth')) return '/';
  return value;
}
