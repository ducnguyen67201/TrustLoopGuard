import { IconArrowLeft } from '@tabler/icons-react';

import { AppLayout } from '@/components/AppLayout';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { InfoHint } from '@/components/ui/info-hint';
import { PageHeader } from '@/components/ui/page-header';
import { readWorkspaceSlug } from '@/lib/search-params';
import { getAgentsPageData } from '@/lib/server/dashboard-data';

import { PolicyForm } from './PolicyForm';
import { createPolicy } from './actions';

export default async function NewPolicyPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const params = await searchParams;
  const workspaceSlug = readWorkspaceSlug(params);
  const data = await getAgentsPageData(workspaceSlug);
  const policiesHref = `/policies?workspace=${data.activeWorkspace.slug}`;

  return (
    <AppLayout
      title="New protection rule"
      workspaceSlug={workspaceSlug}
      breadcrumbs={[{ label: 'Protection rules', href: policiesHref }, { label: 'New rule' }]}
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
            initialValues={initialPolicyValues(params)}
          />
        </Card>
      </div>
    </AppLayout>
  );
}

function readSearchString(value: string | string[] | undefined): string | undefined {
  const raw = Array.isArray(value) ? value[0] : value;
  const trimmed = raw?.trim();
  return trimmed ? trimmed : undefined;
}

function initialPolicyValues(params: Record<string, string | string[] | undefined>) {
  const values: {
    description?: string;
    policyKey?: string;
    sourceYaml?: string;
    severity?: 'low' | 'medium' | 'high' | 'critical';
    action?: 'deny' | 'transform' | 'require_approval';
    enabled?: boolean;
  } = {};
  const description = readSearchString(params['description']);
  const policyKey = readSearchString(params['policyKey']);
  const sourceYaml = readSearchString(params['sourceYaml']);
  const severity = readSearchString(params['severity']);
  const action = readSearchString(params['action']);
  const enabled = readSearchString(params['enabled']);
  if (description !== undefined) values.description = description;
  if (policyKey !== undefined) values.policyKey = policyKey;
  if (sourceYaml !== undefined) values.sourceYaml = sourceYaml;
  if (
    severity === 'low' ||
    severity === 'medium' ||
    severity === 'high' ||
    severity === 'critical'
  ) {
    values.severity = severity;
  }
  if (action === 'deny' || action === 'transform' || action === 'require_approval')
    values.action = action;
  if (enabled !== undefined) values.enabled = enabled === 'true';
  return values;
}
