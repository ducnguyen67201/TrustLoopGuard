import { redirect } from 'next/navigation';
import { revalidatePath } from 'next/cache';

import { AppLayout } from '@/components/AppLayout';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { readWorkspaceSlug } from '@/lib/search-params';
import { getAgentsPageData, getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';

export default async function NewPolicyPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getAgentsPageData(workspaceSlug);

  return (
    <AppLayout title="New policy" workspaceSlug={workspaceSlug}>
      <div className="grid gap-4 px-4 lg:px-6">
        <div className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="outline" className="rounded-sm">
              {data.activeWorkspace.name}
            </Badge>
            <Badge variant="outline" className="rounded-sm">
              draft then enable
            </Badge>
          </div>
          <h2 className="text-2xl font-semibold">Create policy</h2>
        </div>

        <Card className="max-w-3xl">
          <CardHeader>
            <CardTitle>Policy details</CardTitle>
            <CardDescription>
              Create a workspace policy. It will appear in the policy table after save.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form action={createPolicy} className="grid gap-5">
              <input type="hidden" name="workspaceSlug" value={data.activeWorkspace.slug} />
              <Field label="Policy key" htmlFor="policy-key">
                <Input id="policy-key" name="policyKey" placeholder="refund-guarantee" required />
              </Field>

              <Field label="Description" htmlFor="description">
                <Textarea
                  id="description"
                  name="description"
                  placeholder="Block promises that guarantee refunds without approved policy context."
                  required
                />
              </Field>

              <div className="grid gap-4 md:grid-cols-3">
                <Field label="Agent" htmlFor="agent-id">
                  <Select name="agentId">
                    <SelectTrigger id="agent-id" className="w-full">
                      <SelectValue placeholder="Global policy" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="global">Global policy</SelectItem>
                      {data.agents.map((agent) => (
                        <SelectItem key={agent.id} value={agent.id}>
                          {agent.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>

                <Field label="Severity" htmlFor="severity">
                  <Select name="severity" defaultValue="medium" required>
                    <SelectTrigger id="severity" className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="low">Low</SelectItem>
                      <SelectItem value="medium">Medium</SelectItem>
                      <SelectItem value="high">High</SelectItem>
                      <SelectItem value="critical">Critical</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>

                <Field label="Action" htmlFor="action">
                  <Select name="action" defaultValue="block" required>
                    <SelectTrigger id="action" className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="block">Block</SelectItem>
                      <SelectItem value="rewrite">Rewrite</SelectItem>
                      <SelectItem value="escalate">Escalate</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
              </div>

              <div className="flex items-center justify-between gap-3 border p-3">
                <div>
                  <Label htmlFor="enabled">Enabled</Label>
                  <p className="text-sm text-muted-foreground">
                    Disabled policies are saved for review but not active.
                  </p>
                </div>
                <Switch id="enabled" name="enabled" value="true" />
              </div>

              <Field label="Source YAML" htmlFor="source-yaml">
                <Textarea
                  id="source-yaml"
                  name="sourceYaml"
                  placeholder={'id: refund-guarantee\nmatch:\n  literal: "guaranteed refund"\naction: block'}
                  className="min-h-40 font-mono text-sm"
                />
              </Field>

              <div className="flex justify-end gap-2">
                <Button variant="outline" type="button" asChild>
                  <a href={`/policies?workspace=${data.activeWorkspace.slug}`}>Cancel</a>
                </Button>
                <Button type="submit">Create policy</Button>
              </div>
            </form>
          </CardContent>
        </Card>
      </div>
    </AppLayout>
  );
}

async function createPolicy(formData: FormData) {
  'use server';

  const workspaceSlug = readOptionalString(formData, 'workspaceSlug');
  const shell = await getDashboardShell(workspaceSlug);
  const policyKey = readRequiredString(formData, 'policyKey');
  const description = readRequiredString(formData, 'description');
  const severity = readEnum(formData, 'severity', ['low', 'medium', 'high', 'critical'] as const);
  const action = readEnum(formData, 'action', ['block', 'rewrite', 'escalate'] as const);
  const agentId = readOptionalString(formData, 'agentId');
  const sourceYaml =
    readOptionalString(formData, 'sourceYaml') ??
    yamlPolicy(policyKey, description, policyKey, action, severity, agentId === 'global' ? null : agentId);
  const enabled = formData.get('enabled') === 'true';

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

  revalidatePath('/policies');
  redirect(`/policies?workspace=${shell.activeWorkspace.slug}`);
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
description: ${description}
match:
  literal: "${literal}"
action: ${action}
severity: ${severity}
${ownerAgentId ? `owner_agent_id: ${ownerAgentId}\n` : ''}`;
}

function Field({
  label,
  htmlFor,
  children,
}: {
  label: string;
  htmlFor: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
    </div>
  );
}

function readRequiredString(formData: FormData, key: string): string {
  const value = formData.get(key);
  if (typeof value !== 'string' || value.trim() === '') {
    throw new Error(`${key} is required`);
  }
  return value.trim();
}

function readOptionalString(formData: FormData, key: string): string | null {
  const value = formData.get(key);
  if (typeof value !== 'string' || value.trim() === '') return null;
  return value.trim();
}

function readEnum<const T extends readonly string[]>(
  formData: FormData,
  key: string,
  allowed: T,
): T[number] {
  const value = readRequiredString(formData, key);
  if (!allowed.includes(value)) {
    throw new Error(`${key} is invalid`);
  }
  return value;
}
