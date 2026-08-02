#!/usr/bin/env node
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

import { createFeatherlaneAIClient } from './client';
import { createFeatherlaneAIMcpServer } from './server';

async function main(): Promise<void> {
  const server = createFeatherlaneAIMcpServer(createFeatherlaneAIClient());
  await server.connect(new StdioServerTransport());
}

main().catch((error: Error) => {
  console.error(error.message);
  process.exit(1);
});
