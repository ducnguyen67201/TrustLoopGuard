import { Buffer } from 'node:buffer';
import { NextResponse, type NextRequest } from 'next/server';

import { getOptionalDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForWorkspace } from '@/lib/server/tl-client';

type KnowledgeSourceFileResponse = {
  file_name: string;
  media_type: string;
  byte_size: number;
  data_base64: string;
};

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

  let file: KnowledgeSourceFileResponse;
  try {
    file = await rustApiForWorkspace<KnowledgeSourceFileResponse>(
      shell.activeWorkspace.id,
      `/v1/knowledge-sources/${encodeURIComponent(id)}/file`,
    );
  } catch {
    return NextResponse.json({ error: 'file not found' }, { status: 404 });
  }
  const data = Buffer.from(file.data_base64, 'base64');

  return new NextResponse(new Uint8Array(data), {
    headers: {
      'Content-Disposition': `attachment; filename="${escapeHeaderValue(file.file_name)}"`,
      'Content-Length': String(file.byte_size),
      'Content-Type': file.media_type,
    },
  });
}

function escapeHeaderValue(value: string): string {
  return value.replaceAll(/[\r\n]/g, '').replaceAll('\\', '\\\\').replaceAll('"', '\\"');
}
