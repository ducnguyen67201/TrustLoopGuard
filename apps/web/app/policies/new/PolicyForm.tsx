'use client';

import { useActionState, useId } from 'react';
import { useFormStatus } from 'react-dom';
import { IconAlertTriangle, IconLoader2 } from '@tabler/icons-react';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { InfoHint } from '@/components/ui/info-hint';
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
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { VerdictLegend } from '@/components/ui/verdict-legend';

import type { CreatePolicyAction, PolicyFormState } from './policy-form-state';

type AgentOption = { id: string; name: string };

type PolicyFormProps = {
  action: CreatePolicyAction;
  workspaceSlug: string;
  policiesHref: string;
  environmentName: string;
  agents: AgentOption[];
};

export function PolicyForm({
  action,
  workspaceSlug,
  policiesHref,
  environmentName,
  agents,
}: PolicyFormProps) {
  const [state, formAction] = useActionState(action, {});

  return (
    <form action={formAction}>
      <input type="hidden" name="workspaceSlug" value={workspaceSlug} />

      {state.formError ? (
        <CardContent className="pt-0">
          <Alert variant="destructive" aria-live="assertive">
            <IconAlertTriangle aria-hidden />
            <AlertTitle>We couldn&apos;t create this rule</AlertTitle>
            <AlertDescription>{state.formError}</AlertDescription>
          </Alert>
        </CardContent>
      ) : null}

      <CardHeader>
        <CardTitle>Name it</CardTitle>
        <CardDescription>
          What this rule is called and what it stops. These show up in your rules list and in
          decision records.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        <Field
          label="Description"
          name="description"
          hint="A plain-language name for this rule, in one sentence. This is what you'll see in your rules list."
          error={state.fieldErrors?.description}
        >
          {(ids) => (
            <Textarea
              id={ids.control}
              name="description"
              placeholder="Block promises that guarantee refunds without approval."
              required
              aria-invalid={ids.invalid}
              aria-describedby={ids.describedBy}
            />
          )}
        </Field>

        <Field
          label="Rule ID"
          name="policyKey"
          labelHint={<InfoHint term="policyKey" />}
          hint="A short, lowercase id the engine uses — e.g. no-pii. Letters, numbers, and hyphens only. Not the friendly name."
          error={state.fieldErrors?.policyKey}
        >
          {(ids) => (
            <Input
              id={ids.control}
              name="policyKey"
              className="font-mono"
              placeholder="refund-guarantee"
              required
              aria-invalid={ids.invalid}
              aria-describedby={ids.describedBy}
            />
          )}
        </Field>
      </CardContent>

      <Separator />

      <CardHeader>
        <CardTitle>What it does</CardTitle>
        <CardDescription>
          Who this rule applies to, and what the guardrail does when it matches.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        <div className="grid gap-5 md:grid-cols-3">
          <Field
            label="Applies to"
            name="agentId"
            hint="One AI assistant, or all of them."
          >
            {(ids) => (
              <Select name="agentId">
                <SelectTrigger id={ids.control} className="w-full">
                  <SelectValue placeholder="All assistants" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="global">All assistants (global)</SelectItem>
                  {agents.map((agent) => (
                    <SelectItem key={agent.id} value={agent.id}>
                      {agent.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </Field>

          <Field
            label="Severity"
            name="severity"
            labelHint={<InfoHint term="severity" />}
            hint="How serious a match is, from low to critical."
            error={state.fieldErrors?.severity}
          >
            {(ids) => (
              <Select name="severity" defaultValue="medium" required>
                <SelectTrigger id={ids.control} aria-invalid={ids.invalid} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="low">Low</SelectItem>
                  <SelectItem value="medium">Medium</SelectItem>
                  <SelectItem value="high">High</SelectItem>
                  <SelectItem value="critical">Critical</SelectItem>
                </SelectContent>
              </Select>
            )}
          </Field>

          <Field
            label="On a match"
            name="action"
            labelHint={<InfoHint term="verdict" />}
            hint="What the guardrail does when this rule matches."
            error={state.fieldErrors?.action}
          >
            {(ids) => (
              <Select name="action" defaultValue="block" required>
                <SelectTrigger id={ids.control} aria-invalid={ids.invalid} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="block">Block it</SelectItem>
                  <SelectItem value="rewrite">Clean it up (rewrite)</SelectItem>
                  <SelectItem value="escalate">Send for review (escalate)</SelectItem>
                </SelectContent>
              </Select>
            )}
          </Field>
        </div>

        <div className="rounded-md border bg-muted/30 p-4">
          <p className="mb-3 text-xs font-medium text-muted-foreground">
            What each “On a match” choice means
          </p>
          <VerdictLegend verdicts={['rewrite', 'escalate', 'block']} />
        </div>

        <div className="flex items-start justify-between gap-4 rounded-md border bg-muted/40 p-4">
          <div className="grid gap-1">
            <Label htmlFor="enabled">Turn on as soon as I save</Label>
            <p className="text-sm text-muted-foreground">
              Leave off to save it as a draft and review first. Turn on to start checking{' '}
              {environmentName} traffic right away.
            </p>
          </div>
          <Switch id="enabled" name="enabled" value="true" />
        </div>
      </CardContent>

      <Separator />

      <CardHeader>
        <CardTitle>Advanced (optional)</CardTitle>
        <CardDescription>
          Most people can skip this. We&apos;ll build the rule from the fields above unless you
          hand-write it here.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        <details>
          <summary className="cursor-pointer text-sm font-medium text-muted-foreground select-none">
            Write the rule yourself in YAML
          </summary>
          <div className="mt-4">
            <Field
              label="Rule definition (YAML)"
              name="sourceYaml"
              hint="Leave blank to build the rule from the fields above. Only fill this in if you're comfortable with YAML."
              error={state.fieldErrors?.sourceYaml}
            >
              {(ids) => (
                <Textarea
                  id={ids.control}
                  name="sourceYaml"
                  placeholder={'id: refund-guarantee\nmatch:\n  literal: "guaranteed refund"\naction: block'}
                  className="min-h-40 font-mono text-sm"
                  aria-invalid={ids.invalid}
                  aria-describedby={ids.describedBy}
                />
              )}
            </Field>
          </div>
        </details>
      </CardContent>

      <Separator />

      <CardContent className="flex flex-col-reverse gap-2 pt-6 sm:flex-row sm:justify-end">
        <Button variant="outline" type="button" asChild>
          <a href={policiesHref}>Cancel</a>
        </Button>
        <SubmitButton />
      </CardContent>
    </form>
  );
}

function SubmitButton() {
  const { pending } = useFormStatus();
  return (
    <Button type="submit" disabled={pending} aria-disabled={pending}>
      {pending ? (
        <>
          <IconLoader2 className="animate-spin motion-reduce:animate-none" aria-hidden />
          Creating…
        </>
      ) : (
        'Create rule'
      )}
    </Button>
  );
}

type FieldRenderIds = {
  control: string;
  describedBy: string | undefined;
  invalid: boolean;
};

type FieldProps = {
  label: string;
  name: keyof NonNullable<PolicyFormState['fieldErrors']>;
  hint?: string | undefined;
  /** Optional inline help (e.g. an InfoHint) shown next to the label. */
  labelHint?: React.ReactNode;
  error?: string | undefined;
  children: (ids: FieldRenderIds) => React.ReactNode;
};

function Field({ label, name, hint, labelHint, error, children }: FieldProps) {
  const reactId = useId();
  const controlId = `${name}-${reactId}`;
  const hintId = hint ? `${controlId}-hint` : undefined;
  const errorId = error ? `${controlId}-error` : undefined;
  const describedBy = [errorId, hintId].filter(Boolean).join(' ') || undefined;

  return (
    <div className="grid gap-2">
      <Label htmlFor={controlId} className="flex items-center gap-1">
        {label}
        {labelHint}
      </Label>
      {children({ control: controlId, describedBy, invalid: Boolean(error) })}
      {error ? (
        <p id={errorId} role="alert" className="text-sm font-medium text-destructive">
          {error}
        </p>
      ) : null}
      {hint ? (
        <p id={hintId} className="text-sm text-muted-foreground">
          {hint}
        </p>
      ) : null}
    </div>
  );
}
