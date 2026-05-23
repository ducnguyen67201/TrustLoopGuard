export interface AuthRedirectConfig {
  appUrl: string;
  serverUrl: string;
  publicServerUrl: string;
}

export function safeAuthRedirect(url: string, config: AuthRedirectConfig): string {
  const appUrl = new URL(config.appUrl);

  if (url.startsWith('//')) {
    return appUrl.toString();
  }

  if (url.startsWith('/')) {
    return new URL(url, appUrl).toString();
  }

  let target: URL;
  try {
    target = new URL(url);
  } catch {
    return appUrl.toString();
  }

  if (target.origin === appUrl.origin) {
    return target.toString();
  }

  if (isRustOrLocalOrigin(target, config)) {
    return new URL(`${target.pathname}${target.search}${target.hash}`, appUrl).toString();
  }

  return appUrl.toString();
}

function isRustOrLocalOrigin(url: URL, config: AuthRedirectConfig): boolean {
  if (url.port === '8080') return true;
  if (url.hostname === 'localhost' || url.hostname === '127.0.0.1' || url.hostname === '0.0.0.0') {
    return true;
  }

  const serverUrl = new URL(config.serverUrl);
  if (url.origin === serverUrl.origin) return true;

  const publicServerUrl = new URL(config.publicServerUrl);
  return url.origin === publicServerUrl.origin;
}
