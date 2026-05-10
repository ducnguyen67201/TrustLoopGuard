'use client';

import { useState, type ChangeEvent, type FormEvent, type ReactNode } from 'react';
import { z } from 'zod';
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
  proposedOutput:
    "Sure, I promise we'll refund you in full and you have my money-back guarantee.",
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
    return (event: ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
      const next = event.target.value as FormValues[K];
      setValues((prev) => ({ ...prev, [key]: next }));
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
      <form
        onSubmit={onSubmit}
        className="space-y-5 rounded-lg border border-[color:var(--color-border)] bg-[color:var(--color-surface)] p-6"
      >
        <Field label="agent_id" error={fieldErrors.agentId} htmlFor="agentId">
          <input
            id="agentId"
            name="agentId"
            value={values.agentId}
            onChange={update('agentId')}
            className="w-full rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] px-3 py-2 font-mono text-sm outline-none focus:ring-2 focus:ring-[color:var(--color-accent)]/60"
          />
        </Field>

        <Field label="channel" error={fieldErrors.channel} htmlFor="channel">
          <select
            id="channel"
            name="channel"
            value={values.channel}
            onChange={update('channel')}
            className="w-full rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] px-3 py-2 font-mono text-sm outline-none focus:ring-2 focus:ring-[color:var(--color-accent)]/60"
          >
            <option value="chat">chat</option>
            <option value="voice">voice</option>
            <option value="email">email</option>
          </select>
        </Field>

        <Field
          label="policies"
          hint="comma-separated policy ids"
          error={fieldErrors.policies}
          htmlFor="policies"
        >
          <input
            id="policies"
            name="policies"
            value={values.policies}
            onChange={update('policies')}
            className="w-full rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] px-3 py-2 font-mono text-sm outline-none focus:ring-2 focus:ring-[color:var(--color-accent)]/60"
          />
        </Field>

        <Field
          label="input"
          hint="the user prompt sent to the agent"
          error={fieldErrors.input}
          htmlFor="input"
        >
          <textarea
            id="input"
            name="input"
            value={values.input}
            onChange={update('input')}
            rows={3}
            className="w-full rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] px-3 py-2 font-mono text-sm outline-none focus:ring-2 focus:ring-[color:var(--color-accent)]/60"
          />
        </Field>

        <Field
          label="proposed_output"
          hint="the agent draft tl-server should evaluate"
          error={fieldErrors.proposedOutput}
          htmlFor="proposedOutput"
        >
          <textarea
            id="proposedOutput"
            name="proposedOutput"
            value={values.proposedOutput}
            onChange={update('proposedOutput')}
            rows={5}
            className="w-full rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] px-3 py-2 font-mono text-sm outline-none focus:ring-2 focus:ring-[color:var(--color-accent)]/60"
          />
        </Field>

        <button
          type="submit"
          disabled={state.kind === 'loading'}
          className="inline-flex w-full items-center justify-center rounded-md bg-[color:var(--color-accent)] px-4 py-2.5 font-mono text-sm font-medium uppercase tracking-wider text-[color:var(--color-accent-fg)] transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-60"
        >
          {state.kind === 'loading' ? 'checking...' : 'check'}
        </button>
      </form>

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
      <label
        htmlFor={htmlFor}
        className="block font-mono text-xs uppercase tracking-wider text-[color:var(--color-text-muted)]"
      >
        {label}
      </label>
      {children}
      {hint !== undefined && error === undefined ? (
        <p className="text-xs text-[color:var(--color-text-muted)]">{hint}</p>
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
    <aside className="rounded-lg border border-[color:var(--color-border)] bg-[color:var(--color-surface)] p-6">
      <h2 className="font-mono text-xs uppercase tracking-wider text-[color:var(--color-text-muted)]">
        Decision
      </h2>

      {state.kind === 'idle' ? (
        <p className="mt-6 text-sm text-[color:var(--color-text-muted)]">
          Submit a CheckRequest to see the verdict here. tl-server must be running at the URL
          shown in the footer.
        </p>
      ) : null}

      {state.kind === 'loading' ? (
        <p className="mt-6 text-sm text-[color:var(--color-text-muted)]">
          checking with tl-server...
        </p>
      ) : null}

      {state.kind === 'error' ? (
        <div className="mt-6 space-y-2">
          <p className="text-sm text-[color:var(--color-block)]">request failed</p>
          <pre className="overflow-auto rounded-md bg-[color:var(--color-surface-2)] p-3 font-mono text-xs">
            {state.message}
          </pre>
        </div>
      ) : null}

      {state.kind === 'success' ? <DecisionView decision={state.decision} /> : null}
    </aside>
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
          <h3 className="mb-2 font-mono text-xs uppercase tracking-wider text-[color:var(--color-text-muted)]">
            safe_output
          </h3>
          <pre className="overflow-auto rounded-md bg-[color:var(--color-surface-2)] p-3 font-mono text-xs whitespace-pre-wrap">
            {decision.safe_output}
          </pre>
        </section>
      ) : null}

      {decision.triggered_policies.length > 0 ? (
        <section>
          <h3 className="mb-2 font-mono text-xs uppercase tracking-wider text-[color:var(--color-text-muted)]">
            triggered_policies
          </h3>
          <ul className="space-y-2">
            {decision.triggered_policies.map((policy) => (
              <li
                key={`${policy.id}-${policy.severity}-${policy.reason}`}
                className="rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] p-3 text-xs"
              >
                <div className="flex items-center justify-between gap-3 font-mono">
                  <span>{policy.id}</span>
                  <span className="text-[color:var(--color-text-muted)]">{policy.severity}</span>
                </div>
                <p className="mt-1 text-[color:var(--color-text-muted)]">{policy.reason}</p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <footer className="flex items-center justify-between border-t border-[color:var(--color-border)] pt-3 font-mono text-xs text-[color:var(--color-text-muted)]">
        <span>trace_id: {decision.trace_id}</span>
        <span>{decision.latency_ms} ms</span>
      </footer>
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
