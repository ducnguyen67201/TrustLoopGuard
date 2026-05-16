import 'server-only';

import { createHash } from 'node:crypto';
import { eq } from 'drizzle-orm';

import { getDb } from '@/lib/db/client';
import { knowledgeSourceFiles } from '@/lib/db/schema/workspace';

export const MAX_KNOWLEDGE_FILE_BYTES = 10 * 1024 * 1024;

export type KnowledgeFileRecord = {
  knowledgeSourceId: string;
  fileName: string;
  mediaType: string;
  byteSize: number;
  checksumSha256: string;
  data: Buffer;
};

export type PutKnowledgeFileInput = {
  knowledgeSourceId: string;
  fileName: string;
  mediaType: string;
  data: Buffer;
};

export interface KnowledgeFileStore {
  putFile(input: PutKnowledgeFileInput): Promise<KnowledgeFileRecord>;
  getFile(knowledgeSourceId: string): Promise<KnowledgeFileRecord | null>;
  deleteFile(knowledgeSourceId: string): Promise<void>;
}

class DatabaseKnowledgeFileStore implements KnowledgeFileStore {
  async putFile(input: PutKnowledgeFileInput): Promise<KnowledgeFileRecord> {
    const checksumSha256 = createHash('sha256').update(input.data).digest('hex');
    const values = {
      knowledgeSourceId: input.knowledgeSourceId,
      fileName: input.fileName,
      mediaType: input.mediaType,
      byteSize: input.data.byteLength,
      checksumSha256,
      data: input.data,
      updatedAt: new Date(),
    };

    const [row] = await getDb()
      .insert(knowledgeSourceFiles)
      .values(values)
      .onConflictDoUpdate({
        target: knowledgeSourceFiles.knowledgeSourceId,
        set: values,
      })
      .returning();

    if (!row) {
      throw new Error('Could not store knowledge file');
    }

    return row;
  }

  async getFile(knowledgeSourceId: string): Promise<KnowledgeFileRecord | null> {
    const [row] = await getDb()
      .select()
      .from(knowledgeSourceFiles)
      .where(eq(knowledgeSourceFiles.knowledgeSourceId, knowledgeSourceId))
      .limit(1);

    return row ?? null;
  }

  async deleteFile(knowledgeSourceId: string): Promise<void> {
    await getDb()
      .delete(knowledgeSourceFiles)
      .where(eq(knowledgeSourceFiles.knowledgeSourceId, knowledgeSourceId));
  }
}

const databaseKnowledgeFileStore = new DatabaseKnowledgeFileStore();

export function getKnowledgeFileStore(): KnowledgeFileStore {
  return databaseKnowledgeFileStore;
}
