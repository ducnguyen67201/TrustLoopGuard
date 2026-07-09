import { createMDX } from 'fumadocs-mdx/next';
import path from 'node:path';

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  output: 'standalone',
  outputFileTracingRoot: path.resolve(import.meta.dirname, '../../'),
  async rewrites() {
    return [
      {
        source: '/docs/:path*.md',
        destination: '/api/raw-docs/:path*',
      },
    ];
  },
};

export default withMDX(config);
