import type { NextConfig } from 'next';

const config: NextConfig = {
  reactStrictMode: true,
  // Consume @trustloopguard/sdk source-first via the workspace exports map.
  // Next compiles it through SWC instead of expecting a prebuilt dist/.
  transpilePackages: ['@trustloopguard/sdk'],
  typescript: {
    ignoreBuildErrors: false,
  },
};

export default config;
