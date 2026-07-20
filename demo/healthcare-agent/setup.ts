import { HEALTHCARE_AGENT_PROFILE } from './config';
import { HEALTHCARE_POLICY_TEMPLATES } from './policy-templates';
import {
  createHealthcareManagementClient,
  ensureHealthcareRuntimeKey,
  ensureHealthcareWorkspace,
  healthcareWorkspaceAdminConfigFromEnv,
  resolveHealthcareEnvironment,
} from './workspace';

async function main(): Promise<void> {
  const adminConfig = healthcareWorkspaceAdminConfigFromEnv();
  const workspace = await ensureHealthcareWorkspace(adminConfig);
  const environment = await resolveHealthcareEnvironment(workspace.id, adminConfig);
  const client = createHealthcareManagementClient(workspace.id, environment.id, adminConfig);
  process.stdout.write(
    `healthcare workspace ready: ${workspace.name} (${workspace.id}), ${environment.name} (${environment.id})\n`,
  );

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

  const runtimeKey = await ensureHealthcareRuntimeKey(
    workspace.id,
    environment.id,
    adminConfig,
  );
  if (runtimeKey.status === 'created') {
    process.stdout.write(
      `healthcare runtime key created (shown once):\nTL_HEALTHCARE_DEMO_API_KEY=${runtimeKey.plaintextKey}\n`,
    );
    process.stdout.write('store this value in the Marketing deployment secret manager\n');
  } else {
    process.stdout.write(
      `healthcare runtime key ready: ${runtimeKey.apiKey.id} (${runtimeKey.apiKey.prefix}…)\n`,
    );
    if (adminConfig.runtimeApiKey === undefined) {
      process.stdout.write(
        'set TL_HEALTHCARE_DEMO_API_KEY to the existing secret, or rotate it in the dashboard\n',
      );
    }
  }
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : 'unknown setup error';
  process.stderr.write(`healthcare agent setup failed: ${message}\n`);
  process.exitCode = 1;
});
