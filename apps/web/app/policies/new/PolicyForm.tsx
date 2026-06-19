'use client';

import { useActionState, useId } from 'react';
import { useFormStatus } from 'react-dom';
import { IconAlertTriangle, IconLoader2 } from '@tabler/icons-react';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
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
            <AlertTitle>Couldn&apos;t create the policy</AlertTitle>
            <AlertDescription>{state.formError}</AlertDescription>
          </Alert>
        </CardContent>
      ) : null}

      <CardHeader>
        <CardTitle>Identity</CardTitle>
        <CardDescription>
          What this guardrail is called and what it stops. These appear in the policy table and
          decision traces.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        <Field
          label="Policy key"
          name="policyKey"
          hint="Stable identifier. Lowercase, hyphen-separated."
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

        <Field
          label="Description"
          name="description"
          hint="One sentence on the behavior this guardrail prevents."
          error={state.fieldErrors?.description}
        >
          {(ids) => (
            <Textarea
              id={ids.control}
              name="description"
              placeholder="Block promises that guarantee refunds without approved policy context."
              required
              aria-invalid={ids.invalid}
              aria-describedby={ids.describedBy}
            />
          )}
        </Field>
      </CardContent>

      <Separator />

      <CardHeader>
        <CardTitle>Enforcement</CardTitle>
        <CardDescription>
          Who this applies to and what TrustLoopGuard does on a match.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        <div className="grid gap-5 md:grid-cols-3">
          <Field label="Agent" name="agentId" hint="Scope to one agent or all of them.">
            {(ids) => (
              <Select name="agentId">
                <SelectTrigger id={ids.control} className="w-full">
                  <SelectValue placeholder="Global policy" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="global">Global policy</SelectItem>
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
            hint="How serious a match is."
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
            label="Action"
            name="action"
            hint="The verdict on a match."
            error={state.fieldErrors?.action}
          >
            {(ids) => (
              <Select name="action" defaultValue="block" required>
                <SelectTrigger id={ids.control} aria-invalid={ids.invalid} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="block">Block</SelectItem>
                  <SelectItem value="rewrite">Rewrite</SelectItem>
                  <SelectItem value="escalate">Escalate</SelectItem>
                </SelectContent>
              </Select>
            )}
          </Field>
        </div>

        <div className="flex flex-wrap items-center gap-2" aria-hidden>
          <span className="text-xs text-muted-foreground">Verdict legend</span>
          <Badge variant="block">block</Badge>
          <Badge variant="rewrite">rewrite</Badge>
          <Badge variant="escalate">escalate</Badge>
        </div>

        <div className="flex items-start justify-between gap-4 rounded-md border bg-muted/40 p-4">
          <div className="grid gap-1">
            <Label htmlFor="enabled">Enable on save</Label>
            <p className="text-sm text-muted-foreground">
              Off keeps the policy as a draft for review. On enforces it against {environmentName}{' '}
              immediately.
            </p>
          </div>
          <Switch id="enabled" name="enabled" value="true" />
        </div>
      </CardContent>

      <Separator />

      <CardHeader>
        <CardTitle>Definition</CardTitle>
        <CardDescription>
          Optional. Override the generated rule with hand-written policy YAML.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        <Field
          label="Source YAML"
          name="sourceYaml"
          hint="Leave blank to derive the rule from the fields above."
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
        'Create policy'
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
  error?: string | undefined;
  children: (ids: FieldRenderIds) => React.ReactNode;
};

function Field({ label, name, hint, error, children }: FieldProps) {
  const reactId = useId();
  const controlId = `${name}-${reactId}`;
  const hintId = hint ? `${controlId}-hint` : undefined;
  const errorId = error ? `${controlId}-error` : undefined;
  const describedBy = [errorId, hintId].filter(Boolean).join(' ') || undefined;

  return (
    <div className="grid gap-2">
      <Label htmlFor={controlId}>{label}</Label>
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
