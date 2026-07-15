#!/usr/bin/env node
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

import { loadConfig } from './config';
import { createProxy } from './proxy';

async function main(): Promise<void> {
  const proxy = await createProxy(loadConfig());
  const shutdown = (): void => {
    void proxy.close().finally(() => process.exit(0));
  };
  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
  await proxy.server.connect(new StdioServerTransport());
}

main().catch((error: Error) => {
  console.error(error.message);
  process.exit(1);
});
