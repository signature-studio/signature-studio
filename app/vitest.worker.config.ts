import { defineConfig } from 'vitest/config';

// Tests that need `wrangler dev --local`: started by test/worker/run.mjs (npm run test:worker), which passes the URLs in the environment.
export default defineConfig({
  test: { include: ['test/worker/**/*.test.ts'], environment: 'node', testTimeout: 30000, fileParallelism: false },
});
