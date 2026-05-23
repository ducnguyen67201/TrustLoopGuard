export function isAuthSignOutGet(pathname: string): boolean {
  return pathname === '/api/auth/signout';
}

export function authSignOutRedirectUrl(requestUrl: string | URL): URL {
  const redirectUrl = new URL(requestUrl);
  redirectUrl.pathname = '/signout';
  redirectUrl.search = '';
  redirectUrl.hash = '';
  return redirectUrl;
}
