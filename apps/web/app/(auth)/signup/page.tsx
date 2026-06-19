import { redirect } from 'next/navigation';

import { getAuthCapabilities, hasOAuthProvider } from '@/lib/auth-capabilities';

import { AuthScreen } from '../auth-screen';
import { OrDivider } from '../form-feedback';
import { OAuthButtons } from '../signin/oauth-buttons';
import { SignupForm } from './signup-form';

export default async function SignUpPage({
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

  if (!authCapabilities.credentials) {
    redirect(callbackUrl === '/' ? '/signin' : `/signin?callbackUrl=${encodeURIComponent(callbackUrl)}`);
  }

  return (
    <AuthScreen
      eyebrow="Runtime guardrails"
      title="Stand up your first guardrail."
      description="Create an account to wire policies into your agents and watch every decision land in real time."
      cardTitle="Create account"
      cardDescription="Use your workspace identity or create a username."
    >
      {oauthConfigured ? (
        <OAuthButtons callbackUrl={callbackUrl} providers={authCapabilities.oauthProviders} />
      ) : null}
      {oauthConfigured ? <OrDivider /> : null}
      <SignupForm callbackUrl={callbackUrl} />
    </AuthScreen>
  );
}

function safeRedirect(value: string | undefined | null): string {
  if (typeof value !== 'string' || value.trim() === '') return '/';
  if (!value.startsWith('/') || value.startsWith('//')) return '/';
  if (value.startsWith('/signin') || value.startsWith('/signup') || value.startsWith('/api/auth')) {
    return '/';
  }
  return value;
}
