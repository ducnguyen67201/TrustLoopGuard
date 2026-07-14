import path from 'node:path';
import type { NextConfig } from 'next';

const config: NextConfig = {
  reactStrictMode: true,
  typescript: {
    ignoreBuildErrors: false,
  },
  output: 'standalone',
  outputFileTracingRoot: path.resolve(import.meta.dirname, '../../'),
  transpilePackages: ['@trustloopguard/demo', '@trustloopguard/sdk'],
};

export default config;
