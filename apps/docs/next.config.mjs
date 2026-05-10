import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  // Serve raw processed markdown for any docs page at /docs/<path>.md.
  // The .md suffix routes to a thin handler that returns text/markdown.
  // This is the per-page complement to /llms.txt and /llms-full.txt.
  async rewrites() {
    return [
      { source: '/docs.md', destination: '/api/page-md' },
      { source: '/docs/:path*.md', destination: '/api/page-md/:path*' },
    ];
  },
};

export default withMDX(config);
