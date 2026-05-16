'use server';

import { Buffer } from 'node:buffer';
import { eq } from 'drizzle-orm';
import { redirect } from 'next/navigation';
import { revalidatePath } from 'next/cache';

import { getDb } from '@/lib/db/client';
import { knowledgeSources } from '@/lib/db/schema/workspace';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { getKnowledgeFileStore, MAX_KNOWLEDGE_FILE_BYTES } from '@/lib/server/knowledge-file-store';

export async function createKnowledgeSource(formData: FormData) {
  const workspaceSlug = readOptionalString(formData, 'workspaceSlug');
  const shell = await getDashboardShell(workspaceSlug);
  const title = readRequiredString(formData, 'title');
  const kind = readEnum(formData, 'kind', ['url', 'file', 'note'] as const);
  const location = readOptionalString(formData, 'location');
  const notes = readOptionalString(formData, 'notes');
  const file = readOptionalFile(formData, 'file');

  if (kind === 'file' && !file) {
    throw new Error('file is required for File sources');
  }
  if (file && file.size > MAX_KNOWLEDGE_FILE_BYTES) {
    throw new Error('file must be 10 MB or smaller');
  }
  const sourceLocation = kind === 'file' && file ? file.name : location;

  const [source] = await getDb()
    .insert(knowledgeSources)
    .values({
      workspaceId: shell.activeWorkspace.id,
      title,
      kind,
      location: sourceLocation,
      status: 'ready',
      metadata: notes ? { notes } : {},
      lastIndexedAt: new Date(),
    })
    .returning({ id: knowledgeSources.id });

  if (!source) {
    throw new Error('Could not create knowledge source');
  }

  if (kind === 'file' && file) {
    const storedFile = await getKnowledgeFileStore().putFile({
      knowledgeSourceId: source.id,
      fileName: file.name,
      mediaType: file.type || 'application/octet-stream',
      data: Buffer.from(await file.arrayBuffer()),
    });

    await getDb()
      .update(knowledgeSources)
      .set({
        metadata: {
          ...(notes ? { notes } : {}),
          file: {
            fileName: storedFile.fileName,
            mediaType: storedFile.mediaType,
            byteSize: storedFile.byteSize,
            checksumSha256: storedFile.checksumSha256,
          },
        },
        updatedAt: new Date(),
      })
      .where(eq(knowledgeSources.id, source.id));
  }

  revalidatePath('/knowledge-sources');
  redirect(`/knowledge-sources?workspace=${shell.activeWorkspace.slug}`);
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

function readOptionalFile(formData: FormData, key: string): File | null {
  const value = formData.get(key);
  if (!(value instanceof File) || value.size === 0) return null;
  return value;
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
