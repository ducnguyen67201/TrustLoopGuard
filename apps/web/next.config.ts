import type { NextConfig } from 'next';
import { env } from './env';

// Same-domain docs: /docs/* (HTML pages + raw .md endpoints) and the
// agent-readable indexes (/llms.txt, /llms-full.txt) are served by the
// apps/docs deployment but appear under the marketing domain via these
// Next.js rewrites. Set DOCS_ORIGIN to the docs deployment URL in
// production; defaults to the local docs dev port for local development.
const config: NextConfig = {
  reactStrictMode: true,
  // Consume @trustloopguard/sdk source-first via the workspace exports map.
  // Next compiles it through SWC instead of expecting a prebuilt dist/.
  transpilePackages: ['@trustloopguard/sdk'],
  typescript: {
    ignoreBuildErrors: false,
  },
  async rewrites() {
    return [
      { source: '/docs/:path*', destination: `${env.DOCS_ORIGIN}/docs/:path*` },
      { source: '/docs.md', destination: `${env.DOCS_ORIGIN}/docs.md` },
      { source: '/llms.txt', destination: `${env.DOCS_ORIGIN}/llms.txt` },
      { source: '/llms-full.txt', destination: `${env.DOCS_ORIGIN}/llms-full.txt` },
    ];
  },
};

export default config;
