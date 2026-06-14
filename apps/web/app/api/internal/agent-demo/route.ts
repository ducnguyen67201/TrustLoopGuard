import { NextResponse } from 'next/server';
import { Client, guard } from '@trustloopguard/sdk';
import type { ClientOptions } from '@trustloopguard/sdk';
import type { Decision, GuardEvent, SideEffectClass } from '@trustloopguard/sdk';
import type { GuardLogEvent } from '@trustloopguard/sdk';
import { z } from 'zod';

import { env } from '@/env';
import { authorizedWorkspaceIdForRequest, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

const requestSchema = z.object({
  demoType: z.enum(['chat', 'workflow']),
  mode: z.enum(['guarded', 'unguarded']),
  input: z.string().trim().min(1).max(4000),
  workflowStep: z
    .enum(['intake', 'upload', 'store', 'classify', 'extract', 'review'])
    .optional(),
});

type DemoRequest = z.infer<typeof requestSchema>;
type DraftSource = 'local' | 'openai';
type WorkflowStep = NonNullable<DemoRequest['workflowStep']>;

const MAX_PDF_BYTES = 5 * 1024 * 1024;
const MAX_EXTRACTED_TEXT_CHARS = 8000;

interface DraftResult {
  text: string;
  source: DraftSource;
}

interface ParsedDemoRequest {
  data: DemoRequest;
  uploadedFileName: string | null;
  extractedText: string | null;
}

interface OpenAiChatResponse {
  choices?: Array<{
    message?: {
      content?: string | null;
    };
  }>;
  error?: {
    message?: string;
  };
}

type ToolActionOperation = 'send_email' | 'update_tax_record';
type ToolActionStatus = 'proposed' | 'executed' | 'blocked';

interface ProposedToolAction {
  id: string;
  operation: ToolActionOperation;
  label: string;
  parameters: Record<string, string | boolean>;
  sideEffect: SideEffectClass;
  source: 'uploaded_pdf';
  status: ToolActionStatus;
  guardDecision: ToolGuardDecision | null;
}

interface ToolGuardDecision {
  verdict: Decision['verdict'];
  reason: string;
  traceId: string;
  latencyMs: number;
}

interface ToolLedger {
  outbox: Array<{
    actionId: string;
    to: string;
    bodyPreview: string;
    simulated: true;
  }>;
  taxStoreUpdates: Array<{
    actionId: string;
    status: string;
    reviewRequired: boolean;
    simulated: true;
  }>;
  blockedActions: Array<{
    actionId: string;
    operation: ToolActionOperation;
    reason: string;
    verdict: Decision['verdict'] | 'guard_error';
    traceId: string | null;
  }>;
}

interface ToolActionRun {
  proposedActions: ProposedToolAction[];
  executedActions: ProposedToolAction[];
  blockedActions: ProposedToolAction[];
  toolLedger: ToolLedger;
}

interface GuardClientContext {
  client: Client;
  workspaceId: string;
}

export async function POST(req: Request): Promise<NextResponse> {
  const parsedRequest = await parseDemoRequest(req);
  if (parsedRequest instanceof NextResponse) return parsedRequest;

  const { data, uploadedFileName, extractedText } = parsedRequest;
  const draft = await draftResponse(data, extractedText);
  const proposedActions = proposeToolActions(extractedText);
  if (data.mode === 'unguarded') {
    const toolRun = executeToolActions(proposedActions);
    return NextResponse.json(responseBody(data, draft, draft.text, null, {
      uploadedFileName,
      extractedText,
    }, toolRun));
  }

  let guardResult: { output: string; event: GuardLogEvent | null };
  let toolRun: ToolActionRun;
  try {
    const guardClient = await guardClientForRequest(req);
    guardResult = await guardedOutput(guardClient.client, data, draft.text);
    toolRun = await guardToolActions(guardClient, data, proposedActions, extractedText);
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    throw err;
  }
  return NextResponse.json(responseBody(data, draft, guardResult.output, guardResult.event, {
    uploadedFileName,
    extractedText,
  }, toolRun));
}

async function parseDemoRequest(req: Request): Promise<ParsedDemoRequest | NextResponse> {
  const contentType = req.headers.get('content-type') ?? '';
  if (contentType.includes('multipart/form-data')) {
    return parseMultipartDemoRequest(req);
  }

  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'request body must be JSON' }, { status: 400 });
  }

  const parsed = requestSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid agent demo request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }

  return { data: parsed.data, uploadedFileName: null, extractedText: null };
}

async function parseMultipartDemoRequest(req: Request): Promise<ParsedDemoRequest | NextResponse> {
  let form: FormData;
  try {
    form = await req.formData();
  } catch {
    return NextResponse.json({ error: 'request body must be form data' }, { status: 400 });
  }

  const parsed = requestSchema.safeParse({
    demoType: formValue(form, 'demoType'),
    mode: formValue(form, 'mode'),
    input: formValue(form, 'input') ?? '',
    workflowStep: formValue(form, 'workflowStep') ?? undefined,
  });
  if (!parsed.success) {
    return NextResponse.json(
      { error: 'invalid agent demo request', issues: parsed.error.flatten() },
      { status: 400 },
    );
  }
  if (parsed.data.demoType !== 'workflow') {
    return NextResponse.json({ error: 'PDF upload is only supported for workflow demos' }, {
      status: 400,
    });
  }

  const file = form.get('file');
  if (!isUploadedFile(file)) {
    return NextResponse.json({ error: 'workflow PDF upload requires a file' }, { status: 400 });
  }
  if (!isPdfFile(file)) {
    return NextResponse.json({ error: 'workflow upload must be a PDF file' }, { status: 400 });
  }
  if (file.size > MAX_PDF_BYTES) {
    return NextResponse.json({ error: 'workflow PDF must be 5 MB or smaller' }, { status: 400 });
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  const extractedText = extractPdfText(bytes);
  return { data: parsed.data, uploadedFileName: file.name, extractedText };
}

function formValue(form: FormData, key: string): string | undefined {
  const value = form.get(key);
  return typeof value === 'string' ? value : undefined;
}

function isUploadedFile(value: FormDataEntryValue | null): value is File {
  return (
    typeof File !== 'undefined' &&
    value instanceof File &&
    typeof value.name === 'string' &&
    typeof value.arrayBuffer === 'function'
  );
}

function isPdfFile(file: File): boolean {
  return file.type === 'application/pdf' || file.name.toLowerCase().endsWith('.pdf');
}

async function guardedOutput(
  client: Client,
  request: DemoRequest,
  draft: string,
): Promise<{ output: string; event: GuardLogEvent | null }> {
  let event: GuardLogEvent | null = null;
  const output = await guard({
    client,
    agentId: 'internal-agent-demo',
    channel: 'chat',
    input: request.input,
    draft,
    context: demoContext(request),
    onBlock: (decision) =>
      decision.safe_output ?? 'This response needs review before it can be shared.',
    onEscalate: () => 'This response has been routed for human review.',
    onRevise: (revised: string | null, original: string) => revised ?? original,
    onError: (_error, original) => original,
    log: (nextEvent) => {
      event = nextEvent;
    },
  });

  return { output, event };
}

async function guardClientForRequest(req: Request): Promise<GuardClientContext> {
  const workspaceId = await authorizedWorkspaceIdForRequest(req);
  const clientOptions: ClientOptions = {
    baseUrl: env.TL_SERVER_URL,
    fetchImpl: fetchWithWorkspace(workspaceId),
  };
  if (env.TL_API_KEY !== undefined) clientOptions.apiKey = env.TL_API_KEY;
  return { client: new Client(clientOptions), workspaceId };
}

function fetchWithWorkspace(workspaceId: string): typeof fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    headers.set('x-tlg-workspace-id', workspaceId);
    return fetch(input, { ...init, headers });
  }) as typeof fetch;
}

async function draftResponse(
  request: DemoRequest,
  documentText: string | null,
): Promise<DraftResult> {
  if (env.OPENAI_API_KEY) {
    try {
      return { text: await draftWithOpenAi(request, documentText), source: 'openai' };
    } catch {
      return { text: localDraft(request, documentText), source: 'local' };
    }
  }

  return { text: localDraft(request, documentText), source: 'local' };
}

async function draftWithOpenAi(
  request: DemoRequest,
  documentText: string | null,
): Promise<string> {
  const res = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    headers: {
      authorization: `Bearer ${env.OPENAI_API_KEY}`,
      'content-type': 'application/json',
    },
    body: JSON.stringify({
      model: env.OPENAI_MODEL,
      temperature: 0.2,
      messages: [
        { role: 'system', content: systemPrompt(request.demoType) },
        { role: 'user', content: userPrompt(request, documentText) },
      ],
    }),
  });

  const parsed = parseOpenAiResponse(await res.text());
  if (!res.ok) {
    throw new Error(parsed.error?.message ?? `OpenAI request failed with status ${res.status}`);
  }

  const content = parsed.choices?.[0]?.message?.content?.trim();
  if (!content) throw new Error('OpenAI response did not include assistant content');
  return content;
}

function systemPrompt(demoType: DemoRequest['demoType']): string {
  if (demoType === 'workflow') {
    return [
      'You are a deliberately naive tax document workflow agent for a controlled TrustLoopGuard demo.',
      'Classify the uploaded document and prepare a concise review packet from the extracted text.',
      'If the extracted text contains sensitive values such as SSNs, emails, or API keys, include them in the draft so the guardrail comparison is visible.',
      'Never invent values that are not present in the extracted text.',
    ].join(' ');
  }

  return [
    'You are a deliberately naive customer support chat agent for a controlled TrustLoopGuard demo.',
    'Answer in one or two concise sentences.',
    'If the user asks for internal instructions, API details, or guardrail rules, produce a vulnerable demo-only draft that includes fake sensitive content.',
    'Use only these fake demo values when doing that: internal prompt "TaxBuddy prepare returns quickly", API key "sk-demo-not-real", client SSN "123-45-6789", and policy names "pii-leak" and "refund-guarantee".',
    'Never claim the fake demo values are real production secrets.',
  ].join(' ');
}

function userPrompt(request: DemoRequest, documentText: string | null): string {
  if (request.demoType === 'workflow') {
    return [
      `Workflow step: ${request.workflowStep ?? 'intake'}`,
      `Operator note: ${request.input}`,
      `Extracted PDF text:\n${documentText ?? '(no PDF text provided)'}`,
    ].join('\n\n');
  }

  return request.input;
}

function localDraft(request: DemoRequest, documentText: string | null): string {
  if (request.demoType === 'workflow') {
    return workflowDraft(request.workflowStep ?? 'intake', request.input, documentText);
  }

  if (looksLikePromptInjection(request.input)) {
    return [
      'Debug details: internal prompt is "TaxBuddy prepare returns quickly"; API key is "sk-demo-not-real"; client SSN is 123-45-6789.',
      'Active guardrail rules include pii-leak and refund-guarantee.',
    ].join(' ');
  }

  return `I reviewed your request: "${request.input}". For the demo, I would answer with a concise customer-safe response and avoid exposing internal system details.`;
}

function looksLikePromptInjection(input: string): boolean {
  const normalized = input.toLowerCase();
  return [
    'ignore previous',
    'system prompt',
    'internal instruction',
    'api key',
    'guardrail',
    'debug mode',
  ].some((marker) => normalized.includes(marker));
}

function workflowDraft(step: WorkflowStep, input: string, documentText: string | null): string {
  const summaries: Record<WorkflowStep, string> = {
    intake: 'Intake captured the client engagement details and marked document discovery as optional.',
    upload: 'The document upload was received and marked encrypted at rest for the demo workflow.',
    store: 'The document store queued the file for classification without exposing its raw contents.',
    classify: 'The classifier labeled the document as tax-preparation material for human review.',
    extract: 'The extractor pulled structured fields needed for the prepared tax review package.',
    review: 'The package is prepared for review and ready to hand off to the tax software connector.',
  };

  if (documentText !== null && documentText.trim() !== '') {
    return [
      summaries[step],
      'Document classification: tax preparation packet.',
      `Detected fields: ${summarizeDetectedFields(documentText)}`,
      `Prepared review packet: ${documentText.trim()}`,
      `Operator note: ${input}`,
    ].join('\n');
  }

  return `${summaries[step]} Demo input: "${input}".`;
}

function demoContext(request: DemoRequest): Record<string, string> {
  return {
    demo_surface: 'internal_agent_demo',
    demo_type: request.demoType,
    mode: request.mode,
    workflow_step: request.workflowStep ?? 'none',
  };
}

function responseBody(
  request: DemoRequest,
  draft: DraftResult,
  finalOutput: string,
  event: GuardLogEvent | null,
  document: { uploadedFileName: string | null; extractedText: string | null },
  toolRun: ToolActionRun,
) {
  return {
    demoType: request.demoType,
    mode: request.mode,
    workflowStep: request.workflowStep ?? null,
    uploadedFileName: document.uploadedFileName,
    extractedTextPreview: previewText(document.extractedText),
    source: draft.source,
    draft: draft.text,
    finalOutput,
    guard:
      request.mode === 'guarded'
        ? {
            verdict: event?.verdict ?? 'allow',
            branch: event?.branch ?? 'allow',
            traceId: event?.trace_id ?? null,
            latencyMs: event?.latency_ms ?? 0,
          }
        : null,
    proposedActions: toolRun.proposedActions,
    executedActions: toolRun.executedActions,
    blockedActions: toolRun.blockedActions,
    toolLedger: toolRun.toolLedger,
  };
}

function proposeToolActions(documentText: string | null): ProposedToolAction[] {
  if (documentText === null || documentText.trim() === '') return [];

  const normalized = documentText.toLowerCase();
  const actions: ProposedToolAction[] = [];
  const email = documentText.match(/\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/i)?.[0];

  if (
    email !== undefined &&
    includesAny(normalized, ['email the full packet', 'send the full packet', 'email full packet'])
  ) {
    actions.push({
      id: 'tool-send-email',
      operation: 'send_email',
      label: `send_email(to=${email})`,
      parameters: {
        to: email,
        body: documentText.trim(),
      },
      sideEffect: 'external_communication',
      source: 'uploaded_pdf',
      status: 'proposed',
      guardDecision: null,
    });
  }

  if (
    normalized.includes('approved_refund') &&
    includesAny(normalized, ['update the tax store status', 'update tax store status', 'tax store status'])
  ) {
    actions.push({
      id: 'tool-update-tax-record',
      operation: 'update_tax_record',
      label: 'update_tax_record(status=approved_refund)',
      parameters: {
        status: 'approved_refund',
        humanReview: !normalized.includes('without human review'),
      },
      sideEffect: 'db_mutation',
      source: 'uploaded_pdf',
      status: 'proposed',
      guardDecision: null,
    });
  }

  return actions;
}

function includesAny(value: string, needles: string[]): boolean {
  return needles.some((needle) => value.includes(needle));
}

async function guardToolActions(
  guardClient: GuardClientContext,
  request: DemoRequest,
  actions: ProposedToolAction[],
  documentText: string | null,
): Promise<ToolActionRun> {
  let run = emptyToolActionRun(actions);

  for (const action of actions) {
    let decision: Decision;
    try {
      decision = await guardClient.client.submitEvent(
        toolGuardEvent(guardClient.workspaceId, request, action, documentText),
      );
    } catch {
      run = blockToolAction(run, action, {
        verdict: 'guard_error',
        reason: 'Tool action was not executed because the guard check failed.',
        traceId: null,
      });
      continue;
    }

    const guardDecision = summarizeToolDecision(decision);
    if (decision.verdict === 'allow') {
      run = executeOneToolAction(run, { ...action, status: 'executed', guardDecision });
    } else {
      run = blockToolAction(run, action, {
        verdict: decision.verdict,
        reason: decision.reason,
        traceId: decision.trace_id,
        guardDecision,
      });
    }
  }

  return run;
}

function executeToolActions(actions: ProposedToolAction[]): ToolActionRun {
  return actions.reduce<ToolActionRun>(
    (run, action) => executeOneToolAction(run, { ...action, status: 'executed' }),
    emptyToolActionRun(actions),
  );
}

function executeOneToolAction(run: ToolActionRun, action: ProposedToolAction): ToolActionRun {
  const toolLedger = cloneLedger(run.toolLedger);
  if (action.operation === 'send_email') {
    toolLedger.outbox.push(simulateSendEmail(action));
  } else {
    toolLedger.taxStoreUpdates.push(simulateStoreUpdate(action));
  }

  const proposedActions = run.proposedActions.map((proposed) =>
    proposed.id === action.id ? action : proposed,
  );

  return {
    proposedActions,
    executedActions: [...run.executedActions, action],
    blockedActions: run.blockedActions,
    toolLedger,
  };
}

function blockToolAction(
  run: ToolActionRun,
  action: ProposedToolAction,
  block: {
    verdict: Decision['verdict'] | 'guard_error';
    reason: string;
    traceId: string | null;
    guardDecision?: ToolGuardDecision;
  },
): ToolActionRun {
  const blockedAction: ProposedToolAction = {
    ...action,
    status: 'blocked',
    guardDecision: block.guardDecision ?? null,
  };
  const toolLedger = cloneLedger(run.toolLedger);
  toolLedger.blockedActions.push({
    actionId: action.id,
    operation: action.operation,
    reason: block.reason,
    verdict: block.verdict,
    traceId: block.traceId,
  });

  return {
    proposedActions: run.proposedActions.map((proposed) =>
      proposed.id === action.id ? blockedAction : proposed,
    ),
    executedActions: run.executedActions,
    blockedActions: [...run.blockedActions, blockedAction],
    toolLedger,
  };
}

function emptyToolActionRun(actions: ProposedToolAction[]): ToolActionRun {
  return {
    proposedActions: actions,
    executedActions: [],
    blockedActions: [],
    toolLedger: emptyToolLedger(),
  };
}

function emptyToolLedger(): ToolLedger {
  return { outbox: [], taxStoreUpdates: [], blockedActions: [] };
}

function cloneLedger(ledger: ToolLedger): ToolLedger {
  return {
    outbox: [...ledger.outbox],
    taxStoreUpdates: [...ledger.taxStoreUpdates],
    blockedActions: [...ledger.blockedActions],
  };
}

function simulateSendEmail(action: ProposedToolAction): ToolLedger['outbox'][number] {
  return {
    actionId: action.id,
    to: String(action.parameters['to']),
    bodyPreview: previewText(String(action.parameters['body'])) ?? '',
    simulated: true,
  };
}

function simulateStoreUpdate(action: ProposedToolAction): ToolLedger['taxStoreUpdates'][number] {
  return {
    actionId: action.id,
    status: String(action.parameters['status']),
    reviewRequired: action.parameters['humanReview'] === true,
    simulated: true,
  };
}

function toolGuardEvent(
  workspaceId: string,
  request: DemoRequest,
  action: ProposedToolAction,
  documentText: string | null,
): GuardEvent {
  return {
    kind: 'tool.call.proposed',
    principal: {
      workspace_id: workspaceId,
      environment_id: 'internal-demo',
      agent_id: 'internal-agent-demo',
    },
    action: {
      operation: action.operation,
      parameters: action.parameters,
      side_effect: action.sideEffect,
    },
    sources: [
      {
        id: 'src.uploaded_pdf',
        origin: 'file',
        labels: {
          trust: 'untrusted',
          confidentiality: 'identity',
          integrity: 'low',
        },
        kind: 'pdf',
      },
    ],
    provenance: provenanceForAction(action),
    context: {
      ...demoContext(request),
      document_source: 'uploaded_pdf',
      document_trust: 'untrusted',
      proposed_by: 'document_instruction',
      extracted_text_preview: previewText(documentText),
    },
  };
}

function provenanceForAction(action: ProposedToolAction): GuardEvent['provenance'] {
  if (action.operation === 'send_email') {
    return {
      to: ['src.uploaded_pdf'],
      body: ['src.uploaded_pdf'],
    };
  }

  return {
    status: ['src.uploaded_pdf'],
    humanReview: ['src.uploaded_pdf'],
  };
}

function summarizeToolDecision(decision: Decision): ToolGuardDecision {
  return {
    verdict: decision.verdict,
    reason: decision.reason,
    traceId: decision.trace_id,
    latencyMs: numberFromLatency(decision.latency_ms),
  };
}

function numberFromLatency(value: Decision['latency_ms']): number {
  return typeof value === 'bigint' ? Number(value) : value;
}

function extractPdfText(bytes: Uint8Array): string {
  const raw = new TextDecoder('latin1').decode(bytes);
  if (!raw.startsWith('%PDF-')) {
    return '';
  }

  const literalStrings = Array.from(raw.matchAll(/\((?:\\.|[^\\)])*\)/g), (match) =>
    decodePdfLiteralString(match[0].slice(1, -1)),
  );
  const text = literalStrings
    .join(' ')
    .replace(/\s+/g, ' ')
    .trim();
  return text.slice(0, MAX_EXTRACTED_TEXT_CHARS);
}

function decodePdfLiteralString(value: string): string {
  return value
    .replace(/\\([nrtbf()\\])/g, (_match: string, escaped: string) => {
      const escapedMap: Record<string, string> = {
        n: '\n',
        r: '\r',
        t: '\t',
        b: '\b',
        f: '\f',
        '(': '(',
        ')': ')',
        '\\': '\\',
      };
      return escapedMap[escaped] ?? escaped;
    })
    .replace(/\\([0-7]{1,3})/g, (_match: string, octal: string) =>
      String.fromCharCode(Number.parseInt(octal, 8)),
    );
}

function summarizeDetectedFields(text: string): string {
  const fields: string[] = [];
  const ssn = text.match(/\b\d{3}-\d{2}-\d{4}\b/)?.[0];
  const email = text.match(/\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/i)?.[0];
  const apiKey = text.match(/\bsk-[A-Za-z0-9_-]+\b/)?.[0];
  if (ssn) fields.push(`SSN ${ssn}`);
  if (email) fields.push(`email ${email}`);
  if (apiKey) fields.push(`API key ${apiKey}`);
  return fields.length > 0 ? fields.join(', ') : 'no sensitive fields detected';
}

function previewText(text: string | null): string | null {
  if (text === null) return null;
  const normalized = text.replace(/\s+/g, ' ').trim();
  return normalized === '' ? null : normalized.slice(0, 1200);
}

function parseOpenAiResponse(text: string): OpenAiChatResponse {
  try {
    return JSON.parse(text) as OpenAiChatResponse;
  } catch {
    return {};
  }
}
