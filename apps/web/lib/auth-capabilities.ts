import { env, getEnv } from '@/env';

type OAuthProviderAvailability = {
  github: boolean;
  google: boolean;
};

type AuthCapabilities = {
  credentials: boolean;
  oauthProviders: OAuthProviderAvailability;
};

export function isCredentialsAuthEnabled(environment = getEnv()): boolean {
  return environment === 'dev';
}

export function getAuthCapabilities(): AuthCapabilities {
  return {
    credentials: isCredentialsAuthEnabled(),
    oauthProviders: {
      github: Boolean(env.AUTH_GITHUB_ID && env.AUTH_GITHUB_SECRET),
      google: Boolean(env.AUTH_GOOGLE_ID && env.AUTH_GOOGLE_SECRET),
    },
  };
}

export function hasOAuthProvider(providers: OAuthProviderAvailability): boolean {
  return providers.github || providers.google;
}
