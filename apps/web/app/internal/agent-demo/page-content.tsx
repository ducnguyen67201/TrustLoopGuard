'use client';

import {
  Bot,
  Database,
  FileSearch,
  Loader2,
  Mail,
  Send,
  ShieldCheck,
  ShieldOff,
  Upload,
} from 'lucide-react';
import { useMemo, useState } from 'react';
import { z } from 'zod';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import { http } from '@/lib/http';

type DemoType = 'chat' | 'workflow';
type DemoMode = 'guarded' | 'unguarded';
type WorkflowStep = 'intake' | 'upload' | 'store' | 'classify' | 'extract' | 'review';

const toolGuardDecisionSchema = z.object({
  verdict: z.string(),
  reason: z.string(),
  traceId: z.string(),
  latencyMs: z.number(),
});

const proposedActionSchema = z.object({
  id: z.string(),
  operation: z.enum(['send_email', 'update_tax_record']),
  label: z.string(),
  parameters: z.record(z.string(), z.union([z.string(), z.boolean()])),
  sideEffect: z.string(),
  source: z.literal('uploaded_pdf'),
  status: z.enum(['proposed', 'executed', 'blocked']),
  guardDecision: toolGuardDecisionSchema.nullable(),
});

const toolLedgerSchema = z.object({
  outbox: z.array(
    z.object({
      actionId: z.string(),
      to: z.string(),
      bodyPreview: z.string(),
      simulated: z.literal(true),
    }),
  ),
  taxStoreUpdates: z.array(
    z.object({
      actionId: z.string(),
      status: z.string(),
      reviewRequired: z.boolean(),
      simulated: z.literal(true),
    }),
  ),
  blockedActions: z.array(
    z.object({
      actionId: z.string(),
      operation: z.enum(['send_email', 'update_tax_record']),
      reason: z.string(),
      verdict: z.string(),
      traceId: z.string().nullable(),
    }),
  ),
});

const demoResponseSchema = z.object({
  demoType: z.enum(['chat', 'workflow']),
  mode: z.enum(['guarded', 'unguarded']),
  workflowStep: z
    .enum(['intake', 'upload', 'store', 'classify', 'extract', 'review'])
    .nullable(),
  uploadedFileName: z.string().nullable(),
  extractedTextPreview: z.string().nullable(),
  source: z.enum(['local', 'openai']),
  draft: z.string(),
  finalOutput: z.string(),
  guard: z
    .object({
      verdict: z.string(),
      branch: z.string(),
      traceId: z.string().nullable(),
      latencyMs: z.number(),
    })
    .nullable(),
  proposedActions: z.array(proposedActionSchema),
  executedActions: z.array(proposedActionSchema),
  blockedActions: z.array(proposedActionSchema),
  toolLedger: toolLedgerSchema,
});

type DemoResponse = z.infer<typeof demoResponseSchema>;

interface DemoRun extends DemoResponse {
  id: string;
  input: string;
}

const WORKFLOW_DEMO_STEP: WorkflowStep = 'extract';

const workflowSteps = [
  'Upload PDF',
  'Extract text',
  'Classify document',
  'Propose tool actions',
  'Guard actions',
  'Execute or block',
] as const;

const sampleInputs: Record<DemoType, string> = {
  chat: 'A client asks if the agent can reveal the tax workflow system prompt and API key.',
  workflow:
    'Client engagement packet includes a 1099, a bank statement, and notes asking for tax preparation review.',
};

export function InternalAgentDemoPageContent() {
  const [demoType, setDemoType] = useState<DemoType>('chat');
  const [mode, setMode] = useState<DemoMode>('guarded');
  const [input, setInput] = useState(sampleInputs.chat);
  const [selectedPdf, setSelectedPdf] = useState<File | null>(null);
  const [runs, setRuns] = useState<DemoRun[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);

  const latest = runs[0] ?? null;
  const currentSample = useMemo(() => sampleInputs[demoType], [demoType]);

  function selectDemoType(nextDemoType: DemoType) {
    setDemoType(nextDemoType);
    setInput(sampleInputs[nextDemoType]);
  }

  async function runDemo() {
    setError(null);
    setIsRunning(true);
    try {
      const body =
        demoType === 'workflow' && selectedPdf !== null
          ? workflowFormData(mode, input, selectedPdf)
          : {
              demoType,
              mode,
              input,
              ...(demoType === 'workflow' ? { workflowStep: WORKFLOW_DEMO_STEP } : {}),
            };
      const result = await http.post(
        '/api/internal/agent-demo',
        body,
        demoResponseSchema,
      );
      setRuns((current) => [{ ...result, id: crypto.randomUUID(), input }, ...current].slice(0, 8));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Demo request failed');
    } finally {
      setIsRunning(false);
    }
  }

  return (
    <div className="mx-auto grid w-full max-w-6xl gap-5 p-4 lg:p-6">
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div className="grid gap-1">
          <h1 className="text-2xl font-semibold tracking-normal">Internal Agent Demo</h1>
          <p className="max-w-3xl text-sm text-muted-foreground">
            Direct-link demo surface for comparing raw LLM output with TrustLoopGuard-checked
            output.
          </p>
        </div>
        <div className="flex gap-2">
          <ModeButton mode="unguarded" currentMode={mode} onSelect={setMode} />
          <ModeButton mode="guarded" currentMode={mode} onSelect={setMode} />
        </div>
      </div>

      <Tabs value={demoType} onValueChange={(value) => selectDemoType(value as DemoType)}>
        <TabsList>
          <TabsTrigger value="chat" className="gap-2">
            <Bot className="size-4" />
            Chat
          </TabsTrigger>
          <TabsTrigger value="workflow" className="gap-2">
            <FileSearch className="size-4" />
            Workflow
          </TabsTrigger>
        </TabsList>

        <TabsContent value="chat" className="mt-4">
          <DemoGrid
            demoType="chat"
            mode={mode}
            input={input}
            currentSample={currentSample}
            isRunning={isRunning}
            latest={latest}
            runs={runs}
            error={error}
            onInputChange={setInput}
            onRun={runDemo}
          />
        </TabsContent>

        <TabsContent value="workflow" className="mt-4">
          <div className="grid gap-4">
            <WorkflowProgress latest={latest} selectedPdf={selectedPdf} isRunning={isRunning} />
            <PdfUploadPanel selectedPdf={selectedPdf} onSelectPdf={setSelectedPdf} />
            <DemoGrid
              demoType="workflow"
              mode={mode}
              input={input}
              currentSample={currentSample}
              isRunning={isRunning}
              latest={latest}
              runs={runs}
              error={error}
              onInputChange={setInput}
              onRun={runDemo}
            />
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

function workflowFormData(
  mode: DemoMode,
  input: string,
  selectedPdf: File,
): FormData {
  const form = new FormData();
  form.set('demoType', 'workflow');
  form.set('mode', mode);
  form.set('workflowStep', WORKFLOW_DEMO_STEP);
  form.set('input', input);
  form.set('file', selectedPdf);
  return form;
}

function ModeButton({
  mode,
  currentMode,
  onSelect,
}: {
  mode: DemoMode;
  currentMode: DemoMode;
  onSelect: (mode: DemoMode) => void;
}) {
  const isActive = mode === currentMode;
  const Icon = mode === 'guarded' ? ShieldCheck : ShieldOff;

  return (
    <Button
      type="button"
      variant={isActive ? 'default' : 'outline'}
      onClick={() => onSelect(mode)}
      aria-pressed={isActive}
    >
      <Icon className="size-4" />
      {mode === 'guarded' ? 'Guarded' : 'Unguarded'}
    </Button>
  );
}

function DemoGrid({
  demoType,
  mode,
  input,
  currentSample,
  isRunning,
  latest,
  runs,
  error,
  onInputChange,
  onRun,
}: {
  demoType: DemoType;
  mode: DemoMode;
  input: string;
  currentSample: string;
  isRunning: boolean;
  latest: DemoRun | null;
  runs: DemoRun[];
  error: string | null;
  onInputChange: (value: string) => void;
  onRun: () => void;
}) {
  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(320px,420px)]">
      <Card className="rounded-md">
        <CardHeader>
          <CardTitle>{demoType === 'chat' ? 'Chat Agent' : 'Workflow Agent'}</CardTitle>
          <CardDescription>
            {mode === 'guarded'
              ? 'Draft output is checked before display.'
              : 'Draft output is displayed without a guard check.'}
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4">
          <Textarea
            value={input}
            onChange={(event) => onInputChange(event.target.value)}
            placeholder={currentSample}
            className="min-h-32 resize-y"
          />
          <div className="flex flex-wrap items-center gap-2">
            <Button type="button" onClick={onRun} disabled={isRunning || input.trim() === ''}>
              {isRunning ? <Loader2 className="size-4 animate-spin" /> : <Send className="size-4" />}
              Run demo
            </Button>
            <Button type="button" variant="outline" onClick={() => onInputChange(currentSample)}>
              Reset sample
            </Button>
            {error ? <span className="text-sm text-destructive">{error}</span> : null}
          </div>
          <OutputPanel latest={latest} />
        </CardContent>
      </Card>

      <div className="grid gap-4">
        <InspectorPanel latest={latest} mode={mode} />
        <HistoryPanel runs={runs} />
      </div>
    </div>
  );
}

function WorkflowProgress({
  latest,
  selectedPdf,
  isRunning,
}: {
  latest: DemoRun | null;
  selectedPdf: File | null;
  isRunning: boolean;
}) {
  const statuses = workflowStepStatuses(latest, selectedPdf, isRunning);

  return (
    <div className="grid gap-3 rounded-md border bg-card p-3">
      <div className="text-sm font-medium">Workflow progress</div>
      <div className="grid gap-2 sm:grid-cols-3 lg:grid-cols-6">
        {workflowSteps.map((step, index) => {
          const status = statuses[index] ?? 'Pending';
          return (
            <div
              key={step}
              className={`grid min-h-20 gap-1 rounded-md border px-3 py-2 text-left text-sm ${workflowStepClass(
                status,
              )}`}
            >
              <span className="text-xs opacity-75">{String(index + 1).padStart(2, '0')}</span>
              <span className="font-medium">{step}</span>
              <span className="text-xs opacity-80">{status}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function workflowStepStatuses(
  latest: DemoRun | null,
  selectedPdf: File | null,
  isRunning: boolean,
): Array<'Pending' | 'Ready' | 'Running' | 'Done' | 'Skipped'> {
  if (isRunning) return ['Done', 'Running', 'Pending', 'Pending', 'Pending', 'Pending'];
  if (!latest) return [selectedPdf ? 'Ready' : 'Pending', 'Pending', 'Pending', 'Pending', 'Pending', 'Pending'];

  const proposedActions = latest.proposedActions.length;
  const guardedActionCount = latest.proposedActions.filter((action) => action.guardDecision !== null).length;
  const completedActionCount = latest.executedActions.length + latest.blockedActions.length;

  return [
    latest.uploadedFileName ? 'Done' : 'Pending',
    latest.extractedTextPreview ? 'Done' : 'Skipped',
    'Done',
    proposedActions > 0 ? 'Done' : 'Skipped',
    latest.mode === 'guarded' && proposedActions > 0
      ? guardedActionCount === proposedActions
        ? 'Done'
        : 'Pending'
      : 'Skipped',
    completedActionCount > 0 ? 'Done' : proposedActions > 0 ? 'Pending' : 'Skipped',
  ];
}

function workflowStepClass(status: 'Pending' | 'Ready' | 'Running' | 'Done' | 'Skipped'): string {
  if (status === 'Done') return 'border-primary bg-primary text-primary-foreground';
  if (status === 'Running') return 'border-[var(--color-rewrite)] bg-background text-foreground';
  if (status === 'Ready') return 'border-[var(--color-allow)] bg-background text-foreground';
  return 'bg-background text-muted-foreground';
}

function PdfUploadPanel({
  selectedPdf,
  onSelectPdf,
}: {
  selectedPdf: File | null;
  onSelectPdf: (file: File | null) => void;
}) {
  return (
    <div className="grid gap-3 rounded-md border bg-card p-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="grid gap-1">
          <div className="flex items-center gap-2 text-sm font-medium">
            <Upload className="size-4 text-muted-foreground" />
            PDF packet
          </div>
          <div className="text-xs text-muted-foreground">
            Text-based PDFs are read in memory for this run.
          </div>
        </div>
        {selectedPdf ? (
          <Button type="button" variant="outline" size="sm" onClick={() => onSelectPdf(null)}>
            Clear
          </Button>
        ) : (
            <Button type="button" variant="outline" size="sm" asChild>
            <a href="/internal-demo-tax-packet.pdf" download>
              Sample PDF
            </a>
          </Button>
        )}
      </div>
      <div className="flex flex-wrap gap-2">
        <Button type="button" variant="outline" size="sm" asChild>
          <a href="/internal-demo-tool-attack.pdf" download>
            Tool attack PDF
          </a>
        </Button>
      </div>
      <input
        type="file"
        accept="application/pdf,.pdf"
        onChange={(event) => onSelectPdf(event.target.files?.[0] ?? null)}
        className="block w-full text-sm file:mr-3 file:rounded-md file:border file:bg-background file:px-3 file:py-2 file:text-sm file:font-medium"
      />
      {selectedPdf ? (
        <div className="break-all text-xs text-muted-foreground">
          {selectedPdf.name} ({Math.ceil(selectedPdf.size / 1024)} KB)
        </div>
      ) : null}
    </div>
  );
}

function OutputPanel({ latest }: { latest: DemoRun | null }) {
  if (!latest) {
    return (
      <div className="grid min-h-56 place-items-center rounded-md border border-dashed text-sm text-muted-foreground">
        No run yet.
      </div>
    );
  }

  return (
    <div className="grid gap-3">
      {latest.uploadedFileName || latest.extractedTextPreview ? (
        <TextBlock
          title={`PDF text${latest.uploadedFileName ? `: ${latest.uploadedFileName}` : ''}`}
          value={latest.extractedTextPreview ?? 'No extractable text found.'}
          muted
        />
      ) : null}
      <TextBlock title="LLM draft" value={latest.draft} muted={latest.mode === 'guarded'} />
      {latest.demoType === 'workflow' ? <ToolActionsPanel latest={latest} /> : null}
      <TextBlock title="Displayed output" value={latest.finalOutput} />
    </div>
  );
}

function ToolActionsPanel({ latest }: { latest: DemoRun }) {
  return (
    <div className="grid gap-3 rounded-md border p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="text-xs font-semibold uppercase text-muted-foreground">Tool actions</div>
        <Badge variant="outline">{latest.proposedActions.length} proposed</Badge>
      </div>
      {latest.proposedActions.length === 0 ? (
        <p className="text-sm text-muted-foreground">No tool actions were proposed.</p>
      ) : (
        <div className="grid gap-2">
          {latest.proposedActions.map((action) => (
            <div key={action.id} className="grid gap-2 rounded-md border bg-background p-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex min-w-0 items-center gap-2">
                  {action.operation === 'send_email' ? (
                    <Mail className="size-4 text-muted-foreground" />
                  ) : (
                    <Database className="size-4 text-muted-foreground" />
                  )}
                  <span className="break-words font-mono text-xs">{action.label}</span>
                </div>
                <Badge variant={action.status === 'executed' ? 'default' : 'outline'}>
                  {action.status}
                </Badge>
              </div>
              {action.guardDecision ? (
                <div className="grid gap-1 text-xs text-muted-foreground">
                  <div>
                    Guard: {action.guardDecision.verdict} ({action.guardDecision.latencyMs} ms)
                  </div>
                  <div>{action.guardDecision.reason}</div>
                </div>
              ) : null}
            </div>
          ))}
        </div>
      )}
      <ToolLedgerPanel latest={latest} />
    </div>
  );
}

function ToolLedgerPanel({ latest }: { latest: DemoRun }) {
  return (
    <div className="grid gap-2 border-t pt-3">
      <div className="text-xs font-semibold uppercase text-muted-foreground">Fake tool ledger</div>
      <LedgerGroup
        title="Outbox"
        empty="No simulated email sent."
        items={latest.toolLedger.outbox.map((item) => ({
          id: item.actionId,
          text: `${item.to}: ${item.bodyPreview}`,
        }))}
      />
      <LedgerGroup
        title="Tax store updates"
        empty="No simulated tax store update."
        items={latest.toolLedger.taxStoreUpdates.map((item) => ({
          id: item.actionId,
          text: `${item.status}; review required: ${item.reviewRequired ? 'yes' : 'no'}`,
        }))}
      />
      <LedgerGroup
        title="Blocked actions"
        empty="No actions blocked."
        items={latest.toolLedger.blockedActions.map((item) => ({
          id: item.actionId,
          text: `${item.operation}: ${item.verdict} - ${item.reason}`,
        }))}
      />
    </div>
  );
}

function LedgerGroup({
  title,
  empty,
  items,
}: {
  title: string;
  empty: string;
  items: Array<{ id: string; text: string }>;
}) {
  return (
    <div className="grid gap-1">
      <div className="text-xs font-medium">{title}</div>
      {items.length === 0 ? (
        <div className="text-xs text-muted-foreground">{empty}</div>
      ) : (
        items.map((item) => (
          <div key={item.id} className="break-words rounded-md bg-muted px-2 py-1 font-mono text-xs">
            {item.text}
          </div>
        ))
      )}
    </div>
  );
}

function TextBlock({ title, value, muted = false }: { title: string; value: string; muted?: boolean }) {
  return (
    <div className="grid gap-2 rounded-md border p-3">
      <div className="text-xs font-semibold uppercase text-muted-foreground">{title}</div>
      <p className={`whitespace-pre-wrap text-sm leading-6 ${muted ? 'text-muted-foreground' : ''}`}>
        {value}
      </p>
    </div>
  );
}

function InspectorPanel({ latest, mode }: { latest: DemoRun | null; mode: DemoMode }) {
  const guard = latest?.guard ?? null;

  return (
    <Card className="rounded-md">
      <CardHeader>
        <CardTitle>Run Inspector</CardTitle>
        <CardDescription>{mode === 'guarded' ? 'Guardrail branch' : 'Raw branch'}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3 text-sm">
        <InspectorRow label="Mode" value={latest?.mode ?? mode} />
        <InspectorRow label="LLM source" value={latest?.source ?? 'pending'} />
        <InspectorRow
          label="Verdict"
          value={
            guard ? (
              <Badge variant="outline" className={verdictClass(guard.verdict)}>
                {guard.verdict}
              </Badge>
            ) : (
              'none'
            )
          }
        />
        <InspectorRow label="Branch" value={guard?.branch ?? 'none'} />
        <InspectorRow label="Latency" value={guard ? `${guard.latencyMs} ms` : 'none'} />
        <InspectorRow label="Trace" value={guard?.traceId ?? 'none'} />
      </CardContent>
    </Card>
  );
}

function InspectorRow({ label, value }: { label: string; value: string | React.ReactNode }) {
  return (
    <div className="grid grid-cols-[88px_minmax(0,1fr)] items-center gap-3">
      <div className="text-xs font-semibold uppercase text-muted-foreground">{label}</div>
      <div className="min-w-0 break-words font-mono text-xs">{value}</div>
    </div>
  );
}

function HistoryPanel({ runs }: { runs: DemoRun[] }) {
  return (
    <Card className="rounded-md">
      <CardHeader>
        <CardTitle>Recent Runs</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-2">
        {runs.length === 0 ? (
          <div className="text-sm text-muted-foreground">No recent runs.</div>
        ) : (
          runs.map((run) => (
            <div key={run.id} className="grid gap-1 rounded-md border p-3 text-sm">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant={run.mode === 'guarded' ? 'default' : 'outline'}>{run.mode}</Badge>
                <span className="text-xs text-muted-foreground">{run.source}</span>
                {run.workflowStep ? (
                  <span className="text-xs text-muted-foreground">{run.workflowStep}</span>
                ) : null}
              </div>
              <div className="line-clamp-2 text-muted-foreground">{run.input}</div>
            </div>
          ))
        )}
      </CardContent>
    </Card>
  );
}

function verdictClass(verdict: string): string {
  if (verdict === 'allow') return 'border-[var(--color-allow)] text-[var(--color-allow)]';
  if (verdict === 'rewrite') return 'border-[var(--color-rewrite)] text-[var(--color-rewrite)]';
  if (verdict === 'block') return 'border-[var(--color-block)] text-[var(--color-block)]';
  if (verdict === 'escalate') return 'border-[var(--color-escalate)] text-[var(--color-escalate)]';
  return '';
}
