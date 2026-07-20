import { createClient } from '../shared/env';
import { procurementPolicyYaml } from './fixtures';

async function main(): Promise<void> {
  const client = createClient();
  for (const policy of procurementPolicyYaml()) {
    const stored = await client.upsertPolicy(policy.source);
    await client.setPolicyEnabled(stored.id, true);
    process.stdout.write(`procurement policy ready: ${stored.id}\n`);
  }
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  const hint = message.includes('missing bearer token')
    ? '\nSet TL_API_KEY for this local server, then rerun setup.'
    : '';
  process.stderr.write(`procurement agent setup failed: ${message}${hint}\n`);
  process.exitCode = 1;
});
