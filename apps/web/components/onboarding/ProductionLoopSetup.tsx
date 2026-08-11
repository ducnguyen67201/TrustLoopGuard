'use client';

import { IconAlertTriangle, IconCheck, IconCopy, IconRocket } from '@tabler/icons-react';
import Link from 'next/link';
import { useEffect, useMemo, useState, type FormEvent, type ReactNode } from 'react';
import { toast } from 'sonner';
import type {
  CreateGatewayActivationResponse,
  GatewayProductionReadiness,
  GatewayProviderConnection,
  NotificationDeliverySummary,
} from '@featherlane-ai/sdk';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { http } from '@/lib/http';
import { createApiKeyResponseSchema, type CreatedApiKey } from '@/lib/onboarding';
import { useActivationRun } from './useActivationRun';

export function ProductionLoopSetup({
  workspaceSlug,
  environmentId,
  apiBaseUrl,
  agents,
  providerConnections,
  activeRuntimeKeyCount,
}: {
  workspaceSlug: string;
  environmentId: string;
  apiBaseUrl: string;
  agents: Array<{ id: string; name: string }>;
  providerConnections: GatewayProviderConnection[];
  activeRuntimeKeyCount: number;
}) {
  const [providerKind, setProviderKind] = useState<'openai_compatible' | 'anthropic'>(
    'openai_compatible',
  );
  const [providerName, setProviderName] = useState('Production provider');
  const [providerBaseUrl, setProviderBaseUrl] = useState('https://api.openai.com');
  const [providerKey, setProviderKey] = useState('');
  const [model, setModel] = useState('gpt-4o-mini');
  const [fallbackProviderId, setFallbackProviderId] = useState('none');
  const [agentMode, setAgentMode] = useState<'existing' | 'new'>(
    agents.length > 0 ? 'existing' : 'new',
  );
  const [agentId, setAgentId] = useState(agents[0]?.id ?? '');
  const [agentName, setAgentName] = useState('Production agent');
  const [purpose, setPurpose] = useState('Serve users safely and reliably');
  const [email, setEmail] = useState('');
  const [alertsDeferred, setAlertsDeferred] = useState(false);
  const [privacy, setPrivacy] = useState<'no_body_retention' | 'redacted_only' | 'raw_allowed'>(
    'no_body_retention',
  );
  const [privacyConfirmed, setPrivacyConfirmed] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [creatingKey, setCreatingKey] = useState(false);
  const [sendingTest, setSendingTest] = useState(false);
  const [snippetLanguage, setSnippetLanguage] = useState<'node' | 'python'>('node');
  const [activation, setActivation] = useState<CreateGatewayActivationResponse | null>(null);
  const [activationFailure, setActivationFailure] = useState<{
    message: string;
    step: string | undefined;
    readyResourceIds: string[];
  } | null>(null);
  const [createdKey, setCreatedKey] = useState<CreatedApiKey | null>(null);
  const [testDelivery, setTestDelivery] = useState<NotificationDeliverySummary | null>(null);
  const [verificationSessionId, setVerificationSessionId] = useState<string | null>(null);
  const { run, evaluationComplete } = useActivationRun(
    verificationSessionId,
    workspaceSlug,
    environmentId,
  );

  const compatibleFallbacks = providerConnections.filter(
    (provider) => provider.kind === providerKind,
  );

  const contextQuery = new URLSearchParams({
    workspace: workspaceSlug,
    environment: environmentId,
  }).toString();

  const snippet = useMemo(() => {
    if (activation === null || verificationSessionId === null) return '';
    const base = apiBaseUrl.replace(/\/$/, '');
    const gatewayBase = `${base}/v1/gateway/${activation.route.id}/${
      providerKind === 'anthropic' ? 'anthropic' : 'openai'
    }`;
    const quotedBase = JSON.stringify(gatewayBase);
    const quotedModel = JSON.stringify(model);
    const quotedSession = JSON.stringify(verificationSessionId);

    if (snippetLanguage === 'python') {
      const packageName = providerKind === 'anthropic' ? 'anthropic' : 'openai';
      const clientName = providerKind === 'anthropic' ? 'Anthropic' : 'OpenAI';
      const request =
        providerKind === 'anthropic'
          ? `client.messages.create(\n    model=${quotedModel},\n    max_tokens=16,\n    messages=[{"role": "user", "content": "Reply with OK"}],\n    extra_headers=headers,\n)`
          : `client.chat.completions.create(\n    model=${quotedModel},\n    messages=[{"role": "user", "content": "Reply with OK"}],\n    extra_headers=headers,\n)`;
      return `import os\nfrom ${packageName} import ${clientName}\n\nclient = ${clientName}(\n    api_key=os.environ["FEATHERLANE_AI_API_KEY"],\n    base_url=${quotedBase},\n)\nheaders = {\n    "X-Featherlane-Session-Id": ${quotedSession},\n    "X-Featherlane-Session-End": "true",\n}\n${request}`;
    }

    const packageName = providerKind === 'anthropic' ? '@anthropic-ai/sdk' : 'openai';
    const clientName = providerKind === 'anthropic' ? 'Anthropic' : 'OpenAI';
    const request =
      providerKind === 'anthropic'
        ? `client.messages.create({\n  model: ${quotedModel},\n  max_tokens: 16,\n  messages: [{ role: 'user', content: 'Reply with OK' }],\n}, { headers });`
        : `client.chat.completions.create({\n  model: ${quotedModel},\n  messages: [{ role: 'user', content: 'Reply with OK' }],\n}, { headers });`;
    return `import ${clientName} from '${packageName}';\n\nconst client = new ${clientName}({\n  apiKey: process.env.FEATHERLANE_AI_API_KEY,\n  baseURL: ${quotedBase},\n});\nconst headers = {\n  'X-Featherlane-Session-Id': ${quotedSession},\n  'X-Featherlane-Session-End': 'true',\n};\nawait ${request}`;
  }, [activation, apiBaseUrl, model, providerKind, snippetLanguage, verificationSessionId]);

  useEffect(() => {
    if (testDelivery === null || ['sent', 'failed'].includes(testDelivery.status)) return;
    let cancelled = false;
    const timer = setInterval(() => {
      void fetch(`/api/notification-deliveries?${contextQuery}`, { cache: 'no-store' })
        .then((response) => response.json())
        .then((payload: { deliveries?: NotificationDeliverySummary[] }) => {
          if (cancelled) return;
          const updated = payload.deliveries?.find((delivery) => delivery.id === testDelivery.id);
          if (updated !== undefined) setTestDelivery(updated);
        })
        .catch(() => undefined);
    }, 2_000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [contextQuery, testDelivery]);

  async function refreshReadiness(routeId: string, externalId: string) {
    const params = new URLSearchParams(contextQuery);
    params.set('external_id', externalId);
    const response = await fetch(
      `/api/gateway/routes/${encodeURIComponent(routeId)}/production-readiness?${params.toString()}`,
      { cache: 'no-store' },
    );
    if (!response.ok) return;
    const readiness = (await response.json()) as GatewayProductionReadiness;
    setActivation((current) => (current === null ? null : { ...current, readiness }));
  }

  async function createRuntimeKey(current: CreateGatewayActivationResponse) {
    if (creatingKey) return;
    setCreatingKey(true);
    try {
      const key = await http.withoutWorkspace.post(
        `/api/api-keys?${contextQuery}`,
        {
          name: `${current.agent_id} Gateway key`,
          environment_id: environmentId,
          principal_id: current.agent_id,
        },
        createApiKeyResponseSchema,
      );
      setCreatedKey(key);
      await refreshReadiness(current.route.id, current.verification_session_id);
      toast.success('Runtime key created — copy it now');
    } catch (error) {
      toast.error(
        error instanceof Error
          ? `Configuration is saved, but key creation failed: ${error.message}`
          : 'Configuration is saved, but key creation failed',
      );
    } finally {
      setCreatingKey(false);
    }
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    try {
      const nextVerificationSessionId = verificationSessionId ?? crypto.randomUUID();
      setVerificationSessionId(nextVerificationSessionId);
      const response = await fetch(`/api/gateway/activations?${contextQuery}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider: {
            display_name: providerName.trim(),
            kind: providerKind,
            base_url: providerBaseUrl.trim(),
            default_model: model.trim(),
            provider_api_key: providerKey.trim(),
          },
          agent:
            agentMode === 'existing'
              ? { mode: 'existing', agent_id: agentId }
              : { mode: 'new', name: agentName.trim(), purpose: purpose.trim() },
          route_display_name: `${providerName.trim()} production loop`,
          alert_email: email.trim(),
          alerts_deferred: alertsDeferred,
          verification_session_id: nextVerificationSessionId,
          data_handling_mode: privacy,
          confirm_workspace_privacy_change: privacyConfirmed,
          reliability_mode: 'standard',
          fallback_provider_connection_id:
            fallbackProviderId === 'none' ? undefined : fallbackProviderId,
        }),
      });
      const payload = (await response.json()) as CreateGatewayActivationResponse & {
        message?: string;
        details?: {
          activation_step?: string;
          ready_resource_ids?: string[];
        };
      };
      if (!response.ok) {
        const message = payload.message ?? 'Activation failed';
        setActivationFailure({
          message,
          step: payload.details?.activation_step,
          readyResourceIds: payload.details?.ready_resource_ids ?? [],
        });
        throw new Error(message);
      }
      setProviderKey('');
      setActivationFailure(null);
      setActivation(payload);
      setVerificationSessionId(payload.verification_session_id);
      toast.success('Production loop configured');
      if (activeRuntimeKeyCount === 0) await createRuntimeKey(payload);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Activation failed');
    } finally {
      setSubmitting(false);
    }
  }

  async function sendTestEmail() {
    if (
      activation?.notification_rule === undefined ||
      activation.notification_rule === null ||
      sendingTest
    )
      return;
    setSendingTest(true);
    try {
      const response = await fetch(
        `/api/notification-rules/${encodeURIComponent(activation.notification_rule.id)}/test?${contextQuery}`,
        { method: 'POST' },
      );
      const payload = (await response.json()) as {
        deliveries?: NotificationDeliverySummary[];
        message?: string;
      };
      if (!response.ok) throw new Error(payload.message ?? 'Test delivery could not be queued');
      setTestDelivery(payload.deliveries?.[0] ?? null);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Test delivery could not be queued');
    } finally {
      setSendingTest(false);
    }
  }

  useEffect(() => {
    if (activation === null || verificationSessionId === null || run === null) return;
    void refreshReadiness(activation.route.id, verificationSessionId);
  }, [activation?.route.id, evaluationComplete, run?.id, run?.status, verificationSessionId]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <IconRocket />
          Activate production loop
        </CardTitle>
        <CardDescription>
          Connect capture, bounded sessions, evaluation, reliability, privacy, and email alerts in
          one Rust-owned setup.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        {activation === null ? (
          <form className="grid gap-4 md:grid-cols-2" onSubmit={submit}>
            {activationFailure !== null ? (
              <Alert className="md:col-span-2">
                <IconAlertTriangle />
                <AlertTitle>Activation paused</AlertTitle>
                <AlertDescription>
                  {activationFailure.message}
                  {activationFailure.step !== undefined
                    ? ` Resume from ${activationFailure.step}.`
                    : ' Resume the activation.'}
                  {activationFailure.readyResourceIds.length > 0
                    ? ` Already configured: ${activationFailure.readyResourceIds.join(', ')}.`
                    : ''}
                </AlertDescription>
              </Alert>
            ) : null}
            <Field label="Provider name">
              <Input
                required
                value={providerName}
                onChange={(event) => setProviderName(event.target.value)}
              />
            </Field>
            <Field label="Provider">
              <Select
                value={providerKind}
                onValueChange={(value) => {
                  const kind = value as typeof providerKind;
                  setProviderKind(kind);
                  setProviderBaseUrl(
                    kind === 'anthropic' ? 'https://api.anthropic.com' : 'https://api.openai.com',
                  );
                  setModel(kind === 'anthropic' ? 'claude-3-5-sonnet-latest' : 'gpt-4o-mini');
                  setFallbackProviderId('none');
                }}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="openai_compatible">OpenAI-compatible</SelectItem>
                  <SelectItem value="anthropic">Anthropic</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field label="Provider base URL">
              <Input
                required
                type="url"
                value={providerBaseUrl}
                onChange={(event) => setProviderBaseUrl(event.target.value)}
              />
            </Field>
            <Field label="Provider key">
              <Input
                required
                type="password"
                autoComplete="new-password"
                value={providerKey}
                onChange={(event) => setProviderKey(event.target.value)}
              />
            </Field>
            <Field label="Default model">
              <Input required value={model} onChange={(event) => setModel(event.target.value)} />
            </Field>
            <Field label="Fallback provider">
              <Select value={fallbackProviderId} onValueChange={setFallbackProviderId}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">No fallback</SelectItem>
                  {compatibleFallbacks.map((provider) => (
                    <SelectItem key={provider.id} value={provider.id}>
                      {provider.display_name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field label="Agent choice">
              <Select
                value={agentMode}
                onValueChange={(value) => setAgentMode(value as typeof agentMode)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {agents.length > 0 ? (
                    <SelectItem value="existing">Existing agent</SelectItem>
                  ) : null}
                  <SelectItem value="new">New agent</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            {agentMode === 'existing' ? (
              <Field label="Agent">
                <Select value={agentId} onValueChange={setAgentId}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {agents.map((agent) => (
                      <SelectItem key={agent.id} value={agent.id}>
                        {agent.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
            ) : (
              <>
                <Field label="Agent name">
                  <Input
                    required
                    value={agentName}
                    onChange={(event) => setAgentName(event.target.value)}
                  />
                </Field>
                <Field label="One-line purpose">
                  <Input
                    required
                    value={purpose}
                    onChange={(event) => setPurpose(event.target.value)}
                  />
                </Field>
              </>
            )}
            <Field label="Alert email">
              <Input
                required={!alertsDeferred}
                disabled={alertsDeferred}
                type="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />
            </Field>
            <label className="flex items-start gap-2 text-sm md:col-span-2">
              <Checkbox
                checked={alertsDeferred}
                onCheckedChange={(value) => setAlertsDeferred(value === true)}
              />
              <span>
                Set up alerts later. Production readiness will remain needs attention until email
                delivery is configured.
              </span>
            </label>
            <Field label="Privacy">
              <Select
                value={privacy}
                onValueChange={(value) => setPrivacy(value as typeof privacy)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="no_body_retention">Metadata only</SelectItem>
                  <SelectItem value="redacted_only">Verified redaction only</SelectItem>
                  <SelectItem value="raw_allowed">Allow bodies</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Alert className="md:col-span-2">
              <IconAlertTriangle />
              <AlertTitle>Workspace-wide privacy choice</AlertTitle>
              <AlertDescription>
                This changes how all new runtime evidence in this workspace is persisted.
              </AlertDescription>
            </Alert>
            <label className="flex items-start gap-2 text-sm md:col-span-2">
              <Checkbox
                checked={privacyConfirmed}
                onCheckedChange={(value) => setPrivacyConfirmed(value === true)}
              />
              <span>I understand and confirm this workspace-wide persistence setting.</span>
            </label>
            <div className="md:col-span-2">
              <Button disabled={submitting || !privacyConfirmed}>
                {submitting
                  ? 'Activating…'
                  : activationFailure === null
                    ? 'Activate'
                    : 'Resume activation'}
              </Button>
            </div>
          </form>
        ) : (
          <div className="grid gap-4">
            <div className="grid gap-2 sm:grid-cols-2">
              {activation.readiness.checks.map((check) => (
                <div key={check.id} className="flex items-start gap-2 rounded-lg border p-3">
                  {check.ready ? (
                    <IconCheck className="text-success" />
                  ) : (
                    <IconAlertTriangle className="text-warning" />
                  )}
                  <div>
                    <p className="font-medium">{check.label}</p>
                    {check.detail !== null ? (
                      <p className="text-sm text-muted-foreground">{check.detail}</p>
                    ) : null}
                  </div>
                </div>
              ))}
            </div>
            {createdKey !== null ? (
              <Alert>
                <IconCheck />
                <AlertTitle>Copy your runtime key now</AlertTitle>
                <AlertDescription className="grid gap-2">
                  <code className="break-all rounded-lg bg-muted p-3">
                    {createdKey.plaintext_key}
                  </code>
                  <Button
                    type="button"
                    variant="outline"
                    className="w-fit"
                    onClick={() =>
                      void navigator.clipboard
                        .writeText(createdKey.plaintext_key)
                        .then(() => toast.success('Runtime key copied'))
                    }
                  >
                    <IconCopy />
                    Copy key
                  </Button>
                </AlertDescription>
              </Alert>
            ) : activeRuntimeKeyCount === 0 ? (
              <Button
                type="button"
                variant="outline"
                className="w-fit"
                disabled={creatingKey}
                onClick={() => void createRuntimeKey(activation)}
              >
                {creatingKey ? 'Creating key…' : 'Create runtime key'}
              </Button>
            ) : null}
            <div className="grid gap-2">
              <Label>Send exact verification traffic</Label>
              <div className="flex gap-2">
                <Button
                  type="button"
                  size="sm"
                  variant={snippetLanguage === 'node' ? 'secondary' : 'outline'}
                  aria-pressed={snippetLanguage === 'node'}
                  onClick={() => setSnippetLanguage('node')}
                >
                  Node
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant={snippetLanguage === 'python' ? 'secondary' : 'outline'}
                  aria-pressed={snippetLanguage === 'python'}
                  onClick={() => setSnippetLanguage('python')}
                >
                  Python
                </Button>
              </div>
              <pre className="overflow-x-auto rounded-lg bg-muted p-4 text-xs">
                <code>{snippet}</code>
              </pre>
              <Button
                type="button"
                variant="outline"
                className="w-fit"
                onClick={() =>
                  void navigator.clipboard
                    .writeText(snippet)
                    .then(() => toast.success('Snippet copied'))
                }
              >
                <IconCopy />
                Copy snippet
              </Button>
            </div>
            {activation.notification_rule !== undefined && activation.notification_rule !== null ? (
              <div className="flex items-center gap-3">
                <Button
                  type="button"
                  variant="outline"
                  disabled={sendingTest}
                  onClick={() => void sendTestEmail()}
                >
                  {sendingTest ? 'Queuing test…' : 'Send test email'}
                </Button>
                {testDelivery !== null ? (
                  <Badge variant="secondary">Email {testDelivery.status}</Badge>
                ) : null}
              </div>
            ) : (
              <Alert>
                <IconAlertTriangle />
                <AlertTitle>Email alerts deferred</AlertTitle>
                <AlertDescription className="grid gap-2">
                  <span>
                    Resume activation with an alert email before treating this route as production
                    ready. You will re-enter the provider key, which Featherlane never returns.
                  </span>
                  <Button
                    type="button"
                    variant="outline"
                    className="w-fit"
                    onClick={() => {
                      setAlertsDeferred(false);
                      setActivation(null);
                    }}
                  >
                    Resume activation
                  </Button>
                </AlertDescription>
              </Alert>
            )}
            {run !== null ? (
              <Alert>
                {evaluationComplete ? <IconCheck /> : <IconAlertTriangle />}
                <AlertTitle>
                  {evaluationComplete
                    ? 'Exact session finalized and evaluated'
                    : 'Exact session captured — evaluation pending'}
                </AlertTitle>
                <AlertDescription>
                  Run {run.id} is {run.status}.{' '}
                  <Link
                    className="underline underline-offset-4"
                    href={`/runs/${encodeURIComponent(run.id)}?${contextQuery}`}
                  >
                    Open the verified Run
                  </Link>{' '}
                  to inspect evaluation and delivery evidence.
                </AlertDescription>
              </Alert>
            ) : (
              <Badge variant="secondary" className="w-fit">
                Waiting for session {verificationSessionId}
              </Badge>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid gap-2">
      <Label>{label}</Label>
      {children}
    </div>
  );
}
