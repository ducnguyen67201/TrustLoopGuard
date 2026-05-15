import { redirect } from 'next/navigation';
import { revalidatePath } from 'next/cache';
import { and, eq } from 'drizzle-orm';
import { IconArrowRight, IconBuilding, IconKey, IconShieldCheck, IconUsers } from '@tabler/icons-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { getDb } from '@/lib/db/client';
import {
  organizationMembers,
  organizations,
  workspaceMembers,
  workspaces,
  workspaceSettings,
} from '@/lib/db/schema/workspace';
import { getOnboardingUser } from '@/lib/server/dashboard-data';

const guideItems = [
  {
    icon: IconBuilding,
    title: 'Workspace',
    description: 'A workspace is one guarded AI product, app, bot, or customer deployment.',
  },
  {
    icon: IconShieldCheck,
    title: 'Policies',
    description: 'Policies define what your agent should block, rewrite, escalate, or allow.',
  },
  {
    icon: IconUsers,
    title: 'Team',
    description: 'Invite teammates later and give them owner, admin, editor, or viewer access.',
  },
  {
    icon: IconKey,
    title: 'API keys',
    description: 'Runtime keys connect your application to this workspace’s active guardrail config.',
  },
];

export default async function WorkspaceOnboardingPage() {
  const user = await getOnboardingUser();

  return (
    <main className="min-h-screen bg-background px-4 py-8 text-foreground">
      <div className="mx-auto grid w-full max-w-6xl gap-6 lg:grid-cols-[minmax(0,0.9fr)_minmax(420px,0.7fr)]">
        <section className="grid content-start gap-6">
          <div className="grid gap-3">
            <Badge variant="outline" className="w-fit rounded-sm">
              TrustLoopGuard setup
            </Badge>
            <h1 className="text-3xl font-semibold">Create your first workspace</h1>
            <p className="max-w-2xl text-sm leading-6 text-muted-foreground">
              TrustLoopGuard uses workspaces to separate policy, agent, knowledge source, team,
              and API-key settings. Create one workspace for each AI product or deployment you
              want to guard.
            </p>
          </div>

          <div className="grid gap-3 md:grid-cols-2">
            {guideItems.map((item) => (
              <Card key={item.title}>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-base">
                    <item.icon className="size-4 text-primary" />
                    {item.title}
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-sm text-muted-foreground">{item.description}</CardContent>
              </Card>
            ))}
          </div>
        </section>

        <Card>
          <CardHeader>
            <CardDescription>Signed in as {user.email}</CardDescription>
            <CardTitle>Workspace details</CardTitle>
          </CardHeader>
          <CardContent>
            <form action={createWorkspace} className="grid gap-5">
              <Field label="Organization name" htmlFor="organization-name">
                <Input
                  id="organization-name"
                  name="organizationName"
                  defaultValue={suggestOrganizationName(user.email)}
                  required
                />
              </Field>

              <Field label="Workspace name" htmlFor="workspace-name">
                <Input id="workspace-name" name="workspaceName" placeholder="Support AI" required />
              </Field>

              <Field label="What will this workspace guard?" htmlFor="workspace-description">
                <Textarea
                  id="workspace-description"
                  name="description"
                  placeholder="Customer support bot for billing, refunds, and product questions."
                  required
                />
              </Field>

              <div className="border border-dashed p-3 text-sm text-muted-foreground">
                After creation, you can add policies, API keys, agents, documents, and teammates
                from the dashboard sidebar.
              </div>

              <Button type="submit">
                Create workspace
                <IconArrowRight />
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </main>
  );
}

async function createWorkspace(formData: FormData) {
  'use server';

  const user = await getOnboardingUser();
  const organizationName = readRequiredString(formData, 'organizationName');
  const workspaceName = readRequiredString(formData, 'workspaceName');
  const description = readRequiredString(formData, 'description');
  const organizationSlug = slugify(organizationName);
  const workspaceSlug = slugify(workspaceName);
  const organizationId = `org_${organizationSlug}`;
  const workspaceId = `ws_${workspaceSlug}`;
  const db = getDb();

  const [existingOrganization] = await db
    .select({ id: organizations.id })
    .from(organizations)
    .where(eq(organizations.slug, organizationSlug))
    .limit(1);

  const finalOrganizationId = existingOrganization?.id ?? organizationId;

  if (!existingOrganization) {
    await db.insert(organizations).values({
      id: finalOrganizationId,
      name: organizationName,
      slug: organizationSlug,
    });
  }

  await db
    .insert(organizationMembers)
    .values({
      organizationId: finalOrganizationId,
      userId: user.id,
      role: 'owner',
    })
    .onConflictDoUpdate({
      target: [organizationMembers.organizationId, organizationMembers.userId],
      set: {
        role: 'owner',
      },
    });

  await db
    .insert(workspaces)
    .values({
      id: workspaceId,
      organizationId: finalOrganizationId,
      name: workspaceName,
      slug: workspaceSlug,
      description,
    })
    .onConflictDoUpdate({
      target: [workspaces.organizationId, workspaces.slug],
      set: {
        name: workspaceName,
        description,
        updatedAt: new Date(),
        deletedAt: null,
      },
    });

  const [workspace] = await db
    .select({ id: workspaces.id })
    .from(workspaces)
    .where(and(eq(workspaces.organizationId, finalOrganizationId), eq(workspaces.slug, workspaceSlug)))
    .limit(1);

  if (!workspace) {
    throw new Error('workspace creation failed');
  }

  await db
    .insert(workspaceMembers)
    .values({
      workspaceId: workspace.id,
      userId: user.id,
      role: 'owner',
    })
    .onConflictDoUpdate({
      target: [workspaceMembers.workspaceId, workspaceMembers.userId],
      set: {
        role: 'owner',
      },
    });

  await db
    .insert(workspaceSettings)
    .values({
      workspaceId: workspace.id,
      defaultAction: 'allow',
      telemetryEnabled: true,
      retentionDays: '30',
      config: { runtimeMode: 'single-config' },
    })
    .onConflictDoNothing();

  revalidatePath('/');
  revalidatePath('/workspaces');
  redirect('/');
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

function slugify(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48);
}

function suggestOrganizationName(email: string): string {
  const domain = email.split('@')[1]?.split('.')[0];
  if (!domain) return 'My Organization';
  return `${domain.slice(0, 1).toUpperCase()}${domain.slice(1)}`;
}
