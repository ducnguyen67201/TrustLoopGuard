'use server';

import { Buffer } from 'node:buffer';
import { redirect } from 'next/navigation';
import { revalidatePath } from 'next/cache';

import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForUserWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';

const MAX_KNOWLEDGE_FILE_BYTES = 10 * 1024 * 1024;

export async function createKnowledgeSource(formData: FormData) {
  const workspaceSlug = readOptionalString(formData, 'workspaceSlug');
  const shell = await getDashboardShell(workspaceSlug);
  const role = shell.activeWorkspace.role.toLowerCase();
  if (role !== 'owner' && role !== 'admin') {
    throw new WorkspaceAccessError(
      403,
      'workspace owner or admin role is required to create knowledge sources',
    );
  }
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

  await rustApiForUserWorkspace(shell.user, shell.activeWorkspace.id, '/v1/knowledge-sources', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      title,
      kind,
      location: sourceLocation,
      notes,
      file:
        kind === 'file' && file
          ? {
              file_name: file.name,
              media_type: file.type || 'application/octet-stream',
              data_base64: Buffer.from(await file.arrayBuffer()).toString('base64'),
            }
          : null,
    }),
  });

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
