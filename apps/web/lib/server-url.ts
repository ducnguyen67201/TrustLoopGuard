// Centralized lookup of the tl-server base URL. Reads NEXT_PUBLIC_TL_SERVER_URL
// at build time so it's inlined into the browser bundle. Falls back to the
// local dev port that `cargo run -p tl-server` listens on by default.

const DEFAULT_SERVER_URL = 'http://localhost:8080';

export function getServerUrl(): string {
  const fromEnv = process.env['NEXT_PUBLIC_TL_SERVER_URL'];
  if (typeof fromEnv === 'string' && fromEnv.length > 0) {
    return fromEnv;
  }
  return DEFAULT_SERVER_URL;
}
