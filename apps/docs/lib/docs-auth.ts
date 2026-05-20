export const DOCS_AUTH_COOKIE = 'tlg_docs_auth';
export const DOCS_UNLOCK_PATH = '/unlock';

const AUTH_TOKEN_PREFIX = 'trustloopguard-docs-auth:v1:';

export async function createDocsAuthToken(password: string): Promise<string> {
  const input = new TextEncoder().encode(`${AUTH_TOKEN_PREFIX}${password}`);
  const digest = await crypto.subtle.digest('SHA-256', input);

  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('');
}

export function safeDocsRedirectPath(value: FormDataEntryValue | string | null | undefined): string {
  if (typeof value !== 'string' || !value.startsWith('/') || value.startsWith('//')) {
    return '/docs';
  }

  if (value.startsWith(DOCS_UNLOCK_PATH) || value.startsWith('/api/docs-auth')) {
    return '/docs';
  }

  return value;
}
