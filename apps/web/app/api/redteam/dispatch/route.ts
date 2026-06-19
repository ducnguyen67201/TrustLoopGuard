import { Buffer } from 'node:buffer';

import { NextResponse } from 'next/server';
import { z } from 'zod';

import { proxyRustJson } from '@/app/api/_shared';
import { isAllowedAgentTargetUrl } from '@/lib/redteam-core';
import {
  redteamAttackSurfaceSchema,
  redteamJobProfileSchema,
  redteamRunModeSchema,
} from '@/lib/redteam-jobs';

export const runtime = 'nodejs';

const MAX_DOCUMENT_TEMPLATE_BYTES = 10 * 1024 * 1024;

const documentTemplateSchema = z.object({
  file_name: z.string().min(1),
  media_type: z.string().min(1),
  data_base64: z.string().min(1),
  fields: z.record(z.string().min(1), z.string().min(1)).optional(),
  flatten: z.boolean().default(false),
});

// Tailored attack vectors from `redteam/plan`, forwarded as runner seeds.
const workflowPathSchema = z.object({
  source_node: z.string(),
  source_type: z.string(),
  source_category: z.string(),
  sink_node: z.string(),
  sink_type: z.string(),
  sink_category: z.string(),
});
// Mirrors the Rust authoritative cap (`redteam/validation.rs`); every string
// field is forwarded to the runner and stored as JSONB.
const MAX_VECTOR_FIELD_CHARS = 4000;
const attackVectorSchema = z.object({
  goal: z.string().min(1).max(MAX_VECTOR_FIELD_CHARS),
  technique: z.string().min(1).max(MAX_VECTOR_FIELD_CHARS),
  target_operation: z.string().min(1).max(MAX_VECTOR_FIELD_CHARS),
  injection_payload: z.string().min(1).max(MAX_VECTOR_FIELD_CHARS),
  source_path: workflowPathSchema.optional(),
});

// Snake_case wire body forwarded to the Rust orchestrator (RedteamDispatchRequest).
const dispatchBodySchema = z.object({
  target_url: z.string().url(),
  profile: redteamJobProfileSchema,
  mode: redteamRunModeSchema.default('one_off'),
  attack_surface: redteamAttackSurfaceSchema.default('chat'),
  agent_id: z.string().optional(),
  document_template: documentTemplateSchema.optional(),
  attack_vectors: z.array(attackVectorSchema).min(1).max(32).optional(),
});

export async function POST(req: Request): Promise<NextResponse> {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'request body must be JSON' }, { status: 400 });
  }

  const parsed = dispatchBodySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid dispatch request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }

  // SSRF guard: the orchestrator ultimately fetches this target, so only allow
  // loopback agents (deny-by-default; mirrors the arena proxy).
  if (!isAllowedAgentTargetUrl(parsed.data.target_url)) {
    return NextResponse.json(
      { error: 'target_url must target a loopback agent (127.0.0.1 or localhost)' },
      { status: 400 },
    );
  }

  if (parsed.data.document_template !== undefined) {
    if (parsed.data.attack_surface !== 'document_workflow') {
      return NextResponse.json(
        { error: 'document_template is only valid for document_workflow' },
        { status: 400 },
      );
    }
    const templateError = validateDocumentTemplate(parsed.data.document_template);
    if (templateError !== null) {
      return NextResponse.json({ error: templateError }, { status: 400 });
    }
  }

  return proxyRustJson(req, '/v1/redteam/dispatch', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(parsed.data),
  });
}

function validateDocumentTemplate(template: z.infer<typeof documentTemplateSchema>): string | null {
  const looksLikePdf =
    template.file_name.trim().toLowerCase().endsWith('.pdf') ||
    template.media_type.trim().toLowerCase() === 'application/pdf';
  if (!looksLikePdf) return 'document_template must be a PDF';
  if (!isBase64(template.data_base64)) return 'document_template is not base64';
  let bytes: Buffer;
  try {
    bytes = Buffer.from(template.data_base64.trim(), 'base64');
  } catch {
    return 'document_template is not base64';
  }
  if (bytes.length > MAX_DOCUMENT_TEMPLATE_BYTES) {
    return 'document_template must be 10 MB or smaller';
  }
  if (!bytes.subarray(0, 5).equals(Buffer.from('%PDF-'))) {
    return 'document_template must contain PDF bytes';
  }
  return null;
}

function isBase64(value: string): boolean {
  const trimmed = value.trim();
  return trimmed.length % 4 === 0 && /^[A-Za-z0-9+/]*={0,2}$/.test(trimmed);
}
