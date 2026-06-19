import { redirect } from 'next/navigation';
import { revalidatePath } from 'next/cache';
import { IconSparkles } from '@tabler/icons-react';

import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Textarea } from '@/components/ui/textarea';
import { getOnboardingUser } from '@/lib/server/dashboard-data';

import { SetupBrandHeader } from './SetupBrandHeader';
import { SetupGuideRail } from './SetupGuideRail';
import { SetupSubmitButton } from './SetupSubmitButton';

export default async function WorkspaceOnboardingPage() {
  const user = await getOnboardingUser();

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-10 px-4 py-8 lg:px-6 lg:py-12">
        <SetupBrandHeader />

        <div className="grid gap-10 lg:grid-cols-[minmax(0,1fr)_minmax(440px,0.85fr)] lg:gap-16">
          <section className="grid content-start gap-8">
            <div className="grid gap-3">
              <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
                First-run setup
              </p>
              <h1 className="text-balance text-3xl font-semibold tracking-tight sm:text-4xl">
                Create your first workspace
              </h1>
              <p className="max-w-prose text-sm leading-6 text-muted-foreground">
                A workspace keeps the policies, agents, knowledge sources, team, and API
                keys for one guarded deployment together. Set up one workspace per AI
                product you want to put under guard.
              </p>
            </div>

            <Separator />

            <div className="grid gap-4">
              <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
                What lives in a workspace
              </p>
              <SetupGuideRail />
            </div>
          </section>

          <div className="lg:sticky lg:top-12 lg:self-start">
            <Card className="overflow-hidden">
              <CardHeader>
                <CardDescription className="font-mono text-xs">
                  {user.email}
                </CardDescription>
                <CardTitle className="text-xl">Workspace details</CardTitle>
                <CardDescription>
                  You can rename or expand any of this later from the dashboard.
                </CardDescription>
              </CardHeader>
              <form id="create-workspace" action={createWorkspace}>
                <CardContent className="grid gap-5">
                  <Field
                    label="Organization name"
                    htmlFor="organization-name"
                    hint="The company or team this workspace belongs to."
                  >
                    <Input
                      id="organization-name"
                      name="organizationName"
                      defaultValue={suggestOrganizationName(user.email)}
                      required
                    />
                  </Field>

                  <Field
                    label="Workspace name"
                    htmlFor="workspace-name"
                    hint="Name it for the product you’re guarding."
                  >
                    <Input
                      id="workspace-name"
                      name="workspaceName"
                      placeholder="Support AI"
                      required
                    />
                  </Field>

                  <Field
                    label="What will this workspace guard?"
                    htmlFor="workspace-description"
                    hint="A sentence helps your team recognize it later."
                  >
                    <Textarea
                      id="workspace-description"
                      name="description"
                      placeholder="Customer support bot for billing, refunds, and product questions."
                      required
                    />
                  </Field>
                </CardContent>
                <Separator />
                <CardFooter className="flex-col items-stretch gap-3 pt-6">
                  <SetupSubmitButton />
                  <p className="flex items-start gap-2 text-xs leading-5 text-muted-foreground">
                    <IconSparkles className="mt-0.5 size-3.5 shrink-0 text-primary" />
                    Next, add policies, API keys, agents, documents, and teammates from the
                    dashboard sidebar.
                  </p>
                </CardFooter>
              </form>
            </Card>
          </div>
        </div>
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
  const workspaceSlug = slugify(workspaceName);
  void user;
  void organizationName;
  void description;

  revalidatePath('/');
  revalidatePath('/workspaces');
  redirect(`/?workspace=${workspaceSlug}`);
}

function Field({
  label,
  htmlFor,
  hint,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
      {hint !== undefined ? (
        <p className="text-xs leading-5 text-muted-foreground">{hint}</p>
      ) : null}
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
