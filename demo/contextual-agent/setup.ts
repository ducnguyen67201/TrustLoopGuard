import { CONTEXTUAL_AGENT_PROFILE } from './config';
import { CONTEXTUAL_POLICY_TEMPLATES } from './policy-templates';
import {
  createContextualManagementClient,
  disableContextualStarterPolicies,
  ensureContextualRuntimeKey,
  ensureContextualWorkspace,
  contextualWorkspaceAdminConfigFromEnv,
  resolveContextualEnvironment,
} from './workspace';

async function main(): Promise<void> {
  const adminConfig = contextualWorkspaceAdminConfigFromEnv();
  const workspace = await ensureContextualWorkspace(adminConfig);
  const environment = await resolveContextualEnvironment(workspace.id, adminConfig);
  const client = createContextualManagementClient(workspace.id, environment.id, adminConfig);

  process.stdout.write(
    `contextual workspace ready: ${workspace.name} (${workspace.id}), ${environment.name} (${environment.id})\n`,
  );
  const profile = await client.upsertAgent(CONTEXTUAL_AGENT_PROFILE);
  const disabledStarterIds = await disableContextualStarterPolicies(client);
  for (const policyId of disabledStarterIds) {
    process.stdout.write('contextual starter policy disabled: ' + policyId + '\n');
  }
  process.stdout.write(`contextual agent ready: ${profile.agent_id}\n`);

  for (const template of CONTEXTUAL_POLICY_TEMPLATES) {
    const validation = await client.validatePolicy(template.source);
    if (!validation.valid || validation.policy_id !== template.id) {
      throw new Error(`contextual policy validation failed: ${template.id}`);
    }
    const policy = await client.upsertPolicy(template.source);
    await client.setPolicyEnabled(policy.id, true);
    process.stdout.write(`contextual policy enabled: ${policy.id}\n`);
  }

  const runtimeKey = await ensureContextualRuntimeKey(
    workspace.id,
    environment.id,
    adminConfig,
  );
  if (runtimeKey.status === 'created') {
    process.stdout.write(
      `contextual runtime key created (shown once):\nTL_CONTEXTUAL_DEMO_API_KEY=${runtimeKey.plaintextKey}\n`,
    );
    process.stdout.write('store this value in the Marketing deployment secret manager\n');
  } else {
    process.stdout.write(
      `contextual runtime key ready: ${runtimeKey.apiKey.id} (${runtimeKey.apiKey.prefix}…)\n`,
    );
  }
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : 'unknown setup error';
  process.stderr.write(`contextual agent setup failed: ${message}\n`);
  process.exitCode = 1;
});
