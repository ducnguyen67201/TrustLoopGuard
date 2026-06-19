import { redirect } from 'next/navigation';
import { revalidatePath } from 'next/cache';
import { IconAlertTriangle, IconArrowRight } from '@tabler/icons-react';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { InfoHint } from '@/components/ui/info-hint';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Textarea } from '@/components/ui/textarea';
import { getOnboardingUser } from '@/lib/server/dashboard-data';

import { SetupBrandHeader } from './SetupBrandHeader';
import { SetupGuideRail } from './SetupGuideRail';
import { SetupSubmitButton } from './SetupSubmitButton';

// The create-workspace action can fail before it ever redirects (for example a
// product name like "!!!" that slugifies to an empty workspace slug). Rather
// than throw the user into the raw Next error boundary, the action redirects
// back here with a reason and the values they typed, which we surface as an
// inline message and re-fill below.
const WORKSPACE_ERROR_COPY: Record<string, string> = {
  'empty-slug':
    'Please use at least one letter or number in the workspace name — symbols alone won’t work.',
  missing: 'Please fill in every field so we can create your workspace.',
};

type SearchParams = { [key: string]: string | string[] | undefined };

/** Reads a single-value search param, ignoring repeated/array values. */
function firstParam(value: string | string[] | undefined): string | undefined {
  return typeof value === 'string' ? value : undefined;
}

export default async function WorkspaceOnboardingPage({
  searchParams,
}: {
  searchParams: Promise<SearchParams>;
}) {
  const user = await getOnboardingUser();
  const params = await searchParams;
  const errorReason = firstParam(params['error']);
  const errorMessage = errorReason
    ? WORKSPACE_ERROR_COPY[errorReason] ?? WORKSPACE_ERROR_COPY['missing']
    : null;
  const organizationDefault =
    firstParam(params['organizationName']) ?? suggestOrganizationName(user.email);
  const workspaceNameDefault = firstParam(params['workspaceName']) ?? '';
  const descriptionDefault = firstParam(params['description']) ?? '';

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-10 px-4 py-8 lg:px-6 lg:py-12">
        <SetupBrandHeader />

        <div className="grid gap-10 lg:grid-cols-[minmax(0,1fr)_minmax(440px,0.85fr)] lg:gap-16">
          <section className="grid content-start gap-8">
            <div className="grid gap-4">
              <SetupProgress />
              <div className="grid gap-3">
                <h1 className="text-balance text-3xl font-semibold tracking-tight sm:text-4xl">
                  Create your workspace
                </h1>
                <p className="max-w-prose text-sm leading-6 text-muted-foreground">
                  Think of a{' '}
                  <span className="inline-flex items-center gap-1 font-medium text-foreground">
                    workspace
                    <InfoHint term="workspace" />
                  </span>{' '}
                  as a home for one AI product — it keeps that product&apos;s safety rules,
                  AI assistants, and teammates in one place. You only need to fill in three
                  things to get started.
                </p>
              </div>
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
                  Signed in as {user.email}
                </CardDescription>
                <CardTitle className="text-xl">Tell us about your product</CardTitle>
                <CardDescription>
                  Nothing here is permanent — you can rename or change any of it later.
                </CardDescription>
              </CardHeader>
              <form id="create-workspace" action={createWorkspace}>
                <CardContent className="grid gap-5">
                  {errorMessage ? (
                    <Alert variant="destructive" aria-live="assertive">
                      <IconAlertTriangle aria-hidden />
                      <AlertTitle>We couldn&apos;t create your workspace</AlertTitle>
                      <AlertDescription>{errorMessage}</AlertDescription>
                    </Alert>
                  ) : null}

                  <Field
                    label="Organization name"
                    htmlFor="organization-name"
                    required
                    hint="The company or team this belongs to. We’ve filled in a guess — edit if it’s off."
                  >
                    <Input
                      id="organization-name"
                      name="organizationName"
                      defaultValue={organizationDefault}
                      required
                    />
                  </Field>

                  <Field
                    label="Workspace name"
                    htmlFor="workspace-name"
                    required
                    hint="Name it after the AI product you’re protecting, like “Support AI”."
                  >
                    <Input
                      id="workspace-name"
                      name="workspaceName"
                      placeholder="Support AI"
                      defaultValue={workspaceNameDefault}
                      required
                    />
                  </Field>

                  <Field
                    label="What does this product do?"
                    htmlFor="workspace-description"
                    required
                    hint="One sentence in plain words. It just helps your team recognize this later."
                  >
                    <Textarea
                      id="workspace-description"
                      name="description"
                      placeholder="A customer support bot that answers billing, refund, and product questions."
                      defaultValue={descriptionDefault}
                      required
                    />
                  </Field>
                </CardContent>
                <Separator />
                <CardFooter className="flex-col items-stretch gap-4 pt-6">
                  <SetupSubmitButton />
                  <div className="rounded-lg border border-border bg-muted/40 p-3">
                    <p className="flex items-start gap-2 text-xs font-medium text-foreground">
                      <IconArrowRight className="mt-0.5 size-3.5 shrink-0 text-primary" aria-hidden />
                      What happens next
                    </p>
                    <p className="mt-1.5 pl-[1.375rem] text-xs leading-5 text-muted-foreground">
                      We’ll drop you straight into your dashboard, where you can add your
                      first safety rule. It takes about a minute, and we’ll walk you through it.
                    </p>
                  </div>
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
  const organizationName = readOptionalField(formData, 'organizationName');
  const workspaceName = readOptionalField(formData, 'workspaceName');
  const description = readOptionalField(formData, 'description');

  if (!organizationName || !workspaceName || !description) {
    redirect(buildRetryUrl('missing', { organizationName, workspaceName, description }));
  }

  const workspaceSlug = slugify(workspaceName);
  if (workspaceSlug === '') {
    redirect(buildRetryUrl('empty-slug', { organizationName, workspaceName, description }));
  }
  void user;
  void organizationName;
  void description;

  revalidatePath('/');
  revalidatePath('/workspaces');
  redirect(`/?workspace=${workspaceSlug}`);
}

/** Reads a trimmed field value, or an empty string when missing/blank. */
function readOptionalField(formData: FormData, key: string): string {
  const value = formData.get(key);
  return typeof value === 'string' ? value.trim() : '';
}

/**
 * Builds the same-page retry URL: a friendly error reason plus the values the
 * user already typed, so the branded form re-renders with an inline message and
 * nothing they entered is lost.
 */
function buildRetryUrl(
  reason: string,
  values: { organizationName: string; workspaceName: string; description: string },
): string {
  const query = new URLSearchParams({ error: reason });
  if (values.organizationName) query.set('organizationName', values.organizationName);
  if (values.workspaceName) query.set('workspaceName', values.workspaceName);
  if (values.description) query.set('description', values.description);
  return `/onboarding/workspace?${query.toString()}`;
}

function SetupProgress() {
  return (
    <div className="grid gap-2">
      <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
        Step 1 of 2 · Create your workspace
      </p>
      <ol className="flex items-center gap-2" aria-label="Setup progress: step 1 of 2">
        <li
          aria-current="step"
          className="h-1.5 flex-1 rounded-full bg-primary"
          title="Step 1 — Create your workspace"
        />
        <li
          className="h-1.5 flex-1 rounded-full bg-border"
          title="Step 2 — Add your first safety rule"
        />
      </ol>
    </div>
  );
}

function Field({
  label,
  htmlFor,
  hint,
  required = false,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: string;
  required?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <div className="flex items-baseline justify-between gap-2">
        <Label htmlFor={htmlFor}>{label}</Label>
        <span className="text-xs font-medium text-muted-foreground">
          {required ? (
            <>
              Required
              <span aria-hidden className="ml-0.5 text-destructive">
                *
              </span>
            </>
          ) : (
            'Optional'
          )}
        </span>
      </div>
      {children}
      {hint !== undefined ? (
        <p className="text-xs leading-5 text-muted-foreground">{hint}</p>
      ) : null}
    </div>
  );
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
