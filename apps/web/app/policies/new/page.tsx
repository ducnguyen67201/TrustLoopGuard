import { redirect } from 'next/navigation';
import { revalidatePath } from 'next/cache';
import { IconArrowLeft } from '@tabler/icons-react';

import { AppLayout } from '@/components/AppLayout';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { InfoHint } from '@/components/ui/info-hint';
import { PageHeader } from '@/components/ui/page-header';
import { readWorkspaceSlug } from '@/lib/search-params';
import { getAgentsPageData, getDashboardShell } from '@/lib/server/dashboard-data';
import { RustApiError, rustApiForWorkspace } from '@/lib/server/tl-client';

import { PolicyForm } from './PolicyForm';
import type { PolicyFormState } from './policy-form-state';

export default async function NewPolicyPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getAgentsPageData(workspaceSlug);
  const policiesHref = `/policies?workspace=${data.activeWorkspace.slug}`;

  return (
    <AppLayout
      title="New protection rule"
      workspaceSlug={workspaceSlug}
      breadcrumbs={[
        { label: 'Protection rules', href: policiesHref },
        { label: 'New rule' },
      ]}
    >
      <div className="grid gap-6 px-4 lg:px-6">
        <PageHeader
          eyebrow={data.activeWorkspace.name}
          title="Create a protection rule"
          help={<InfoHint term="policy" />}
          description="Set up a rule that watches every request and decides what to do. Save it as a draft to review first, or turn it on to start checking traffic right away."
          actions={
            <Button variant="outline" asChild>
              <a href={policiesHref}>
                <IconArrowLeft />
                Back to rules
              </a>
            </Button>
          }
        />

        <Card className="max-w-3xl">
          <PolicyForm
            action={createPolicy}
            workspaceSlug={data.activeWorkspace.slug}
            policiesHref={policiesHref}
            environmentName={data.activeEnvironment.name}
            agents={data.agents.map((agent) => ({ id: agent.id, name: agent.name }))}
          />
        </Card>
      </div>
    </AppLayout>
  );
}

async function createPolicy(
  _state: PolicyFormState,
  formData: FormData,
): Promise<PolicyFormState> {
  'use server';

  const workspaceSlug = readOptionalString(formData, 'workspaceSlug');
  const shell = await getDashboardShell(workspaceSlug);

  const validation = validatePolicyForm(formData);
  if (!validation.ok) {
    return { fieldErrors: validation.fieldErrors };
  }
  const { policyKey, description, severity, action, agentId } = validation.value;
  const sourceYaml =
    readOptionalString(formData, 'sourceYaml') ??
    yamlPolicy(policyKey, description, policyKey, action, severity, agentId === 'global' ? null : agentId);
  const enabled = formData.get('enabled') === 'true';

  try {
    await rustApiForWorkspace(shell.activeWorkspace.id, '/v1/policies', {
      method: 'POST',
      headers: { 'content-type': 'application/yaml' },
      body: sourceYaml,
    });
    if (!enabled) {
      await rustApiForWorkspace(
        shell.activeWorkspace.id,
        `/v1/policies/${encodeURIComponent(policyKey)}/enabled`,
        {
          method: 'PATCH',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ enabled: false }),
        },
      );
    }
  } catch (error: unknown) {
    return { formError: createPolicyErrorMessage(error) };
  }

  revalidatePath('/policies');
  redirect(`/policies?workspace=${shell.activeWorkspace.slug}`);
}

type ValidatedPolicyForm = {
  policyKey: string;
  description: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  action: 'block' | 'rewrite' | 'escalate';
  agentId: string | null;
};

type PolicyValidationResult =
  | { ok: true; value: ValidatedPolicyForm }
  | { ok: false; fieldErrors: NonNullable<PolicyFormState['fieldErrors']> };

function validatePolicyForm(formData: FormData): PolicyValidationResult {
  const fieldErrors: NonNullable<PolicyFormState['fieldErrors']> = {};

  const policyKey = readOptionalString(formData, 'policyKey');
  if (policyKey === null) {
    fieldErrors.policyKey = 'Policy key is required.';
  } else if (!/^[a-z0-9][a-z0-9-]*$/.test(policyKey)) {
    fieldErrors.policyKey = 'Use lowercase letters, numbers, and hyphens only.';
  }

  const description = readOptionalString(formData, 'description');
  if (description === null) fieldErrors.description = 'Description is required.';

  const severity = readEnumOrNull(formData, 'severity', [
    'low',
    'medium',
    'high',
    'critical',
  ] as const);
  if (severity === null) fieldErrors.severity = 'Choose a severity.';

  const action = readEnumOrNull(formData, 'action', ['block', 'rewrite', 'escalate'] as const);
  if (action === null) fieldErrors.action = 'Choose an action.';

  if (
    policyKey === null ||
    fieldErrors.policyKey !== undefined ||
    description === null ||
    severity === null ||
    action === null
  ) {
    return { ok: false, fieldErrors };
  }

  return {
    ok: true,
    value: {
      policyKey,
      description,
      severity,
      action,
      agentId: readOptionalString(formData, 'agentId'),
    },
  };
}

function createPolicyErrorMessage(error: unknown): string {
  if (error instanceof RustApiError) {
    if (error.status === 409) {
      return 'A policy with this key already exists. Pick a different policy key.';
    }
    if (error.status === 400 || error.status === 422) {
      return 'The policy definition was rejected. Check the source YAML and the fields above, then try again.';
    }
  }
  return 'Something went wrong saving the policy. Please try again.';
}

function yamlPolicy(
  id: string,
  description: string,
  literal: string,
  action: string,
  severity: string,
  ownerAgentId: string | null,
): string {
  return `id: ${id}
description: ${JSON.stringify(description)}
match:
  literal: ${JSON.stringify(literal)}
action: ${action}
severity: ${severity}
${ownerAgentId ? `owner_agent_id: ${JSON.stringify(ownerAgentId)}\n` : ''}`;
}

function readOptionalString(formData: FormData, key: string): string | null {
  const value = formData.get(key);
  if (typeof value !== 'string' || value.trim() === '') return null;
  return value.trim();
}

function readEnumOrNull<const T extends readonly string[]>(
  formData: FormData,
  key: string,
  allowed: T,
): T[number] | null {
  const value = readOptionalString(formData, key);
  if (value === null || !allowed.includes(value)) return null;
  return value;
}
