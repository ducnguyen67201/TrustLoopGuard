import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('packages the public demos and SDK inside the Marketing image', () => {
  const packageJson = JSON.parse(
    readFileSync(new URL('./package.json', import.meta.url), 'utf8'),
  ) as {
    dependencies: Record<string, string>;
  };
  const dockerfile = readFileSync(new URL('./Dockerfile', import.meta.url), 'utf8');
  const dockerignore = readFileSync(new URL('../../.dockerignore', import.meta.url), 'utf8');

  assert.equal(packageJson.dependencies['@featherlane-ai/demo'], 'workspace:*');
  assert.equal(packageJson.dependencies['@featherlane-ai/sdk'], 'workspace:*');
  assert.match(dockerfile, /COPY demo\/package\.json \.\/demo\//);
  assert.match(dockerfile, /COPY sdks\/typescript\/package\.json \.\/sdks\/typescript\//);
  assert.match(dockerfile, /COPY demo \.\/demo/);
  assert.match(dockerfile, /COPY sdks\/typescript \.\/sdks\/typescript/);
  assert.match(
    dockerfile,
    /RUN pnpm --filter @featherlane-ai\/sdk build \\\n && pnpm --filter marketing build/,
  );
  assert.match(dockerignore, /!demo\/shared\/\*\*/);
  assert.match(dockerignore, /!demo\/procurement-agent\/\*\*/);
  assert.match(dockerignore, /!demo\/healthcare-agent\/\*\*/);
  assert.match(dockerignore, /!demo\/stripe-refund-agent\/\*\*/);
  assert.match(dockerignore, /!demo\/contextual-agent\/\*\*/);
});
