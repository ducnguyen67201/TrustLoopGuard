import { and, eq, isNull } from 'drizzle-orm';
import { NextResponse, type NextRequest } from 'next/server';

import { getDb } from '@/lib/db/client';
import { knowledgeSources } from '@/lib/db/schema/workspace';
import { getOptionalDashboardShell } from '@/lib/server/dashboard-data';
import { getKnowledgeFileStore } from '@/lib/server/knowledge-file-store';

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ id?: string }> },
) {
  const { id } = await params;
  if (!id) {
    return NextResponse.json({ error: 'knowledge source id is required' }, { status: 400 });
  }

  const workspaceSlug = request.nextUrl.searchParams.get('workspace');
  const shell = await getOptionalDashboardShell(workspaceSlug);
  if (!shell) {
    return NextResponse.json({ error: 'authentication required' }, { status: 401 });
  }

  const [source] = await getDb()
    .select({ id: knowledgeSources.id, kind: knowledgeSources.kind })
    .from(knowledgeSources)
    .where(
      and(
        eq(knowledgeSources.id, id),
        eq(knowledgeSources.workspaceId, shell.activeWorkspace.id),
        eq(knowledgeSources.kind, 'file'),
        isNull(knowledgeSources.deletedAt),
      ),
    )
    .limit(1);

  if (!source) {
    return NextResponse.json({ error: 'file not found' }, { status: 404 });
  }

  const file = await getKnowledgeFileStore().getFile(source.id);
  if (!file) {
    return NextResponse.json({ error: 'file not found' }, { status: 404 });
  }

  return new NextResponse(new Uint8Array(file.data), {
    headers: {
      'Content-Disposition': `attachment; filename="${escapeHeaderValue(file.fileName)}"`,
      'Content-Length': String(file.byteSize),
      'Content-Type': file.mediaType,
    },
  });
}

function escapeHeaderValue(value: string): string {
  return value.replaceAll(/[\r\n]/g, '').replaceAll('\\', '\\\\').replaceAll('"', '\\"');
}
