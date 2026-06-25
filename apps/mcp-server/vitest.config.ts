import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    coverage: {
      exclude: ['src/index.ts'],
      include: ['src/client.ts', 'src/handlers.ts', 'src/server.ts'],
    },
  },
});
