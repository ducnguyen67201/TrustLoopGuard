#!/usr/bin/env node
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

import { createTrustLoopClient } from './client';
import { createTrustLoopMcpServer } from './server';

async function main(): Promise<void> {
  const server = createTrustLoopMcpServer(createTrustLoopClient());
  await server.connect(new StdioServerTransport());
}

main().catch((error: Error) => {
  console.error(error.message);
  process.exit(1);
});
