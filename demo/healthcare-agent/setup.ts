import { createClient } from '../shared/env';
import {
  HEALTHCARE_AGENT_PROFILE,
  HEALTHCARE_POLICY_TEMPLATES,
} from './config';

async function main(): Promise<void> {
  const client = createClient();
  const profile = await client.upsertAgent(HEALTHCARE_AGENT_PROFILE);
  process.stdout.write(`healthcare agent ready: ${profile.agent_id}\n`);

  for (const template of HEALTHCARE_POLICY_TEMPLATES) {
    const validation = await client.validatePolicy(template.source);
    if (!validation.valid || validation.policy_id !== template.id) {
      throw new Error(`healthcare policy validation failed: ${template.id}`);
    }
    const policy = await client.upsertPolicy(template.source);
    await client.setPolicyEnabled(policy.id, true);
    process.stdout.write(`healthcare policy enabled: ${policy.id}\n`);
  }
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : 'unknown setup error';
  process.stderr.write(`healthcare agent setup failed: ${message}\n`);
  process.exitCode = 1;
});
