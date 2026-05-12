'use client';

import { AlertCircle, Loader2 } from 'lucide-react';
import { useState, type ChangeEvent, type FormEvent, type ReactNode } from 'react';
import { z } from 'zod';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Textarea } from '@/components/ui/textarea';
import { check } from '../../lib/check';
import {
  formSchema,
  toCheckRequest,
  type DecisionResponse,
  type FormValues,
} from '../../lib/schemas';
import { VerdictPill } from './VerdictPill';

const INITIAL_FORM: FormValues = {
  agentId: 'agent-demo-1',
  channel: 'chat',
  policies: 'refund-promise',
  input: 'Can I get a refund for last month?',
  proposedOutput: "Sure, I promise we'll refund you in full and you have my money-back guarantee.",
};

type FieldErrors = Partial<Record<keyof FormValues, string>>;

type SubmitState =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'success'; decision: DecisionResponse }
  | { kind: 'error'; message: string };

export function Playground() {
  const [values, setValues] = useState<FormValues>(INITIAL_FORM);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [state, setState] = useState<SubmitState>({ kind: 'idle' });

  function update<K extends keyof FormValues>(key: K) {
    return (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      const next = event.target.value as FormValues[K];
      setValues((prev) => ({ ...prev, [key]: next }));
    };
  }

  function updateValue<K extends keyof FormValues>(key: K) {
    return (next: string) => {
      setValues((prev) => ({ ...prev, [key]: next as FormValues[K] }));
    };
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFieldErrors({});

    const parsed = formSchema.safeParse(values);
    if (!parsed.success) {
      const next: FieldErrors = {};
      for (const issue of parsed.error.issues) {
        const head = issue.path[0];
        if (typeof head === 'string' && head in INITIAL_FORM) {
          next[head as keyof FormValues] = issue.message;
        }
      }
      setFieldErrors(next);
      return;
    }

    setState({ kind: 'loading' });
    try {
      const decision = await check(toCheckRequest(parsed.data));
      setState({ kind: 'success', decision });
    } catch (error: unknown) {
      setState({ kind: 'error', message: describeError(error) });
    }
  }

  return (
    <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
      <Card>
        <CardHeader>
          <CardTitle>Check request</CardTitle>
          <CardDescription>Compose the payload sent to the guardrail server.</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="space-y-5">
            <Field label="agent_id" error={fieldErrors.agentId} htmlFor="agentId">
              <Input
                id="agentId"
                name="agentId"
                value={values.agentId}
                onChange={update('agentId')}
                className="font-mono"
              />
            </Field>

            <Field label="channel" error={fieldErrors.channel} htmlFor="channel">
              <Select value={values.channel} onValueChange={updateValue('channel')}>
                <SelectTrigger id="channel" className="w-full font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="chat">chat</SelectItem>
                  <SelectItem value="voice">voice</SelectItem>
                  <SelectItem value="email">email</SelectItem>
                </SelectContent>
              </Select>
            </Field>

            <Field
              label="policies"
              hint="comma-separated policy ids"
              error={fieldErrors.policies}
              htmlFor="policies"
            >
              <Input
                id="policies"
                name="policies"
                value={values.policies}
                onChange={update('policies')}
                className="font-mono"
              />
            </Field>

            <Field
              label="input"
              hint="the user prompt sent to the agent"
              error={fieldErrors.input}
              htmlFor="input"
            >
              <Textarea
                id="input"
                name="input"
                value={values.input}
                onChange={update('input')}
                rows={3}
                className="min-h-24 resize-y font-mono"
              />
            </Field>

            <Field
              label="proposed_output"
              hint="the agent draft tl-server should evaluate"
              error={fieldErrors.proposedOutput}
              htmlFor="proposedOutput"
            >
              <Textarea
                id="proposedOutput"
                name="proposedOutput"
                value={values.proposedOutput}
                onChange={update('proposedOutput')}
                rows={5}
                className="min-h-36 resize-y font-mono"
              />
            </Field>

            <Button type="submit" disabled={state.kind === 'loading'} className="w-full">
              {state.kind === 'loading' ? (
                <>
                  <Loader2 className="animate-spin" />
                  Checking
                </>
              ) : (
                'Check'
              )}
            </Button>
          </form>
        </CardContent>
      </Card>

      <ResultPanel state={state} />
    </div>
  );
}

interface FieldProps {
  label: string;
  htmlFor: string;
  hint?: string | undefined;
  error?: string | undefined;
  children: ReactNode;
}

function Field({ label, htmlFor, hint, error, children }: FieldProps) {
  return (
    <div className="space-y-1.5">
      <Label
        htmlFor={htmlFor}
        className="font-mono text-xs uppercase tracking-wider text-muted-foreground"
      >
        {label}
      </Label>
      {children}
      {hint !== undefined && error === undefined ? (
        <p className="text-xs text-muted-foreground">{hint}</p>
      ) : null}
      {error !== undefined ? (
        <p className="text-xs text-[color:var(--color-block)]">{error}</p>
      ) : null}
    </div>
  );
}

interface ResultPanelProps {
  state: SubmitState;
}

function ResultPanel({ state }: ResultPanelProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Decision</CardTitle>
        <CardDescription>Verdict, policy hits, and response metadata.</CardDescription>
      </CardHeader>
      <CardContent>
        {state.kind === 'idle' ? (
          <p className="text-sm text-muted-foreground">
            Submit a CheckRequest to see the verdict here. tl-server must be running at the URL
            shown in the footer.
          </p>
        ) : null}

        {state.kind === 'loading' ? (
          <Alert>
            <Loader2 className="animate-spin" />
            <AlertTitle>Checking</AlertTitle>
            <AlertDescription>Waiting for tl-server to return a decision.</AlertDescription>
          </Alert>
        ) : null}

        {state.kind === 'error' ? (
          <Alert variant="destructive">
            <AlertCircle />
            <AlertTitle>Request failed</AlertTitle>
            <AlertDescription>
              <pre className="mt-2 max-h-64 overflow-auto rounded-md bg-muted p-3 font-mono text-xs text-foreground">
                {state.message}
              </pre>
            </AlertDescription>
          </Alert>
        ) : null}

        {state.kind === 'success' ? <DecisionView decision={state.decision} /> : null}
      </CardContent>
    </Card>
  );
}

interface DecisionViewProps {
  decision: DecisionResponse;
}

function DecisionView({ decision }: DecisionViewProps) {
  return (
    <div className="mt-6 space-y-6">
      <div className="space-y-3">
        <VerdictPill verdict={decision.verdict} />
        <p className="text-sm">{decision.reason}</p>
      </div>

      {decision.safe_output !== null ? (
        <section>
          <h3 className="mb-2 font-mono text-xs uppercase tracking-wider text-muted-foreground">
            safe_output
          </h3>
          <pre className="overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap">
            {decision.safe_output}
          </pre>
        </section>
      ) : null}

      {decision.triggered_policies.length > 0 ? (
        <section>
          <h3 className="mb-2 font-mono text-xs uppercase tracking-wider text-muted-foreground">
            triggered_policies
          </h3>
          <ul className="space-y-2">
            {decision.triggered_policies.map((policy) => (
              <li
                key={`${policy.id}-${policy.severity}-${policy.reason}`}
                className="rounded-md border bg-muted/50 p-3 text-xs"
              >
                <div className="flex items-center justify-between gap-3 font-mono">
                  <span>{policy.id}</span>
                  <Badge variant="outline" className="font-mono">
                    {policy.severity}
                  </Badge>
                </div>
                <p className="mt-1 text-muted-foreground">{policy.reason}</p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <Separator />
      <CardFooter className="justify-between p-0 font-mono text-xs text-muted-foreground">
        <span>trace_id: {decision.trace_id}</span>
        <span>{decision.latency_ms} ms</span>
      </CardFooter>
    </div>
  );
}

function describeError(error: unknown): string {
  if (error instanceof z.ZodError) {
    return `decision schema mismatch:\n${error.issues
      .map((i) => `  - ${i.path.join('.') || '(root)'}: ${i.message}`)
      .join('\n')}`;
  }
  if (error instanceof Error) return error.message;
  return 'unknown error';
}
