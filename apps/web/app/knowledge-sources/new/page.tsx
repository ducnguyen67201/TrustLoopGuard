import { redirect } from 'next/navigation';
import { revalidatePath } from 'next/cache';
import { IconBook2 } from '@tabler/icons-react';

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
import { Textarea } from '@/components/ui/textarea';
import { getDb } from '@/lib/db/client';
import { knowledgeSources } from '@/lib/db/schema/workspace';
import { getDashboardShell } from '@/lib/server/dashboard-data';

export default async function NewKnowledgeSourcePage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getDashboardShell(workspaceSlug);

  return (
    <AppLayout title="Add source" workspaceSlug={workspaceSlug}>
      <div className="grid gap-4 px-4 lg:px-6">
        <div className="grid gap-2">
          <Badge variant="outline" className="w-fit rounded-sm">
            {data.activeWorkspace.name}
          </Badge>
          <h2 className="flex items-center gap-2 text-2xl font-semibold">
            <IconBook2 className="size-5 text-primary" />
            Add knowledge source
          </h2>
          <p className="max-w-2xl text-sm text-muted-foreground">
            Knowledge sources are workspace-owned documents, URLs, or notes that provide approved
            context for guardrail decisions and authority checks.
          </p>
        </div>

        <Card className="max-w-3xl">
          <CardHeader>
            <CardDescription>Workspace document context</CardDescription>
            <CardTitle>Source details</CardTitle>
          </CardHeader>
          <CardContent>
            <form action={createKnowledgeSource} className="grid gap-5">
              <input type="hidden" name="workspaceSlug" value={data.activeWorkspace.slug} />

              <Field label="Title" htmlFor="title">
                <Input id="title" name="title" placeholder="Refund policy" required />
              </Field>

              <div className="grid gap-4 md:grid-cols-2">
                <Field label="Kind" htmlFor="kind">
                  <Select name="kind" defaultValue="url" required>
                    <SelectTrigger id="kind" className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="url">URL</SelectItem>
                      <SelectItem value="file">File</SelectItem>
                      <SelectItem value="note">Note</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>

                <Field label="Location" htmlFor="location">
                  <Input
                    id="location"
                    name="location"
                    placeholder="https://example.com/help/refunds"
                  />
                </Field>
              </div>

              <Field label="Notes" htmlFor="notes">
                <Textarea
                  id="notes"
                  name="notes"
                  placeholder="What should the team know about this source?"
                />
              </Field>

              <div className="border border-dashed p-3 text-sm text-muted-foreground">
                V1 stores source metadata in the workspace. Indexing can be attached later; new
                sources start as draft unless they are simple notes or URLs.
              </div>

              <div className="flex justify-end gap-2">
                <Button variant="outline" type="button" asChild>
                  <a href={`/knowledge-sources?workspace=${data.activeWorkspace.slug}`}>Cancel</a>
                </Button>
                <Button type="submit">Add source</Button>
              </div>
            </form>
          </CardContent>
        </Card>
      </div>
    </AppLayout>
  );
}

async function createKnowledgeSource(formData: FormData) {
  'use server';

  const workspaceSlug = readOptionalString(formData, 'workspaceSlug');
  const shell = await getDashboardShell(workspaceSlug);
  const title = readRequiredString(formData, 'title');
  const kind = readEnum(formData, 'kind', ['url', 'file', 'note'] as const);
  const location = readOptionalString(formData, 'location');
  const notes = readOptionalString(formData, 'notes');

  await getDb().insert(knowledgeSources).values({
    workspaceId: shell.activeWorkspace.id,
    title,
    kind,
    location,
    status: kind === 'file' ? 'draft' : 'ready',
    metadata: notes ? { notes } : {},
    lastIndexedAt: kind === 'file' ? null : new Date(),
  });

  revalidatePath('/knowledge-sources');
  redirect(`/knowledge-sources?workspace=${shell.activeWorkspace.slug}`);
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

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
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
