'use client';

import { IconArrowRight, IconCheck, IconCopy } from '@tabler/icons-react';
import Link from 'next/link';
import { useState, type FormEvent } from 'react';
import { toast } from 'sonner';

import { CopyBlock } from '@/components/onboarding/CopyBlock';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { http } from '@/lib/http';
import {
  assistantOptions,
  buildAssistantPrompt,
  buildSdkSnippet,
  createApiKeyResponseSchema,
  sanitizeAgentId,
  type AssistantKind,
  type CreatedApiKey,
} from '@/lib/onboarding';

/**
 * Builds the ?workspace=&environment= suffix for onboarding-internal links so
 * the selected context survives the whole flow (web-ui-conventions.md).
 * API calls don't need this — lib/http.ts appends both automatically.
 */
export function onboardingContextQuery(
  workspaceSlug: string,
  requestedEnvironmentId?: string,
): string {
  const context = new URLSearchParams();
  if (workspaceSlug !== '') context.set('workspace', workspaceSlug);
  if (requestedEnvironmentId !== undefined && requestedEnvironmentId !== '') {
    context.set('environment', requestedEnvironmentId);
  }
  const query = context.toString();
  return query === '' ? '' : `?${query}`;
}

/**
 * Onboarding step 2: create an API key (one-time reveal) and hand the user
 * the two integration paths — the SDK quick-start and a paste-into-your-AI-
 * assistant prompt. The plaintext key lives only in component state; it is
 * never placed in URLs, storage, or the snippet strings.
 */
export function ConnectAgentStep({
  baseUrl,
  environmentId,
  defaultAgentId,
  workspaceSlug,
  requestedEnvironmentId,
}: {
  baseUrl: string;
  environmentId: string;
  defaultAgentId: string;
  workspaceSlug: string;
  requestedEnvironmentId?: string | undefined;
}) {
  const [agentId, setAgentId] = useState(defaultAgentId);
  const [submitting, setSubmitting] = useState(false);
  const [created, setCreated] = useState<CreatedApiKey | null>(null);
  const [copied, setCopied] = useState(false);
  const [assistant, setAssistant] = useState<AssistantKind>('claude');

  const contextQuery = onboardingContextQuery(workspaceSlug, requestedEnvironmentId);
  const cleanAgentId =
    sanitizeAgentId(agentId).replace(/^-+|-+$/g, '') || defaultAgentId;
  const selectedAssistant = assistantOptions.find((option) => option.id === assistant);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    try {
      const data = await http.post(
        '/api/api-keys',
        { name: `${cleanAgentId} key`, environment_id: environmentId },
        createApiKeyResponseSchema,
      );
      setCreated(data);
    } catch (err) {
      toast.error(
        err instanceof Error
          ? `Couldn't create the key: ${err.message}. Please try again.`
          : "Couldn't create the key. Please check your connection and try again.",
      );
    } finally {
      setSubmitting(false);
    }
  }

  async function copyKey() {
    if (created === null) return;
    try {
      await navigator.clipboard.writeText(created.plaintext_key);
      setCopied(true);
      toast.success('Key copied');
    } catch {
      toast.error('Copy failed. Select the key text and copy it manually before leaving.');
    }
  }

  if (created === null) {
    return (
      <form onSubmit={onSubmit} className="grid gap-5">
        <div className="grid gap-2">
          <Label htmlFor="onboarding-agent-id">Name your agent</Label>
          <Input
            id="onboarding-agent-id"
            required
            autoComplete="off"
            className="font-mono"
            value={agentId}
            onChange={(event) => setAgentId(sanitizeAgentId(event.target.value))}
            aria-describedby="onboarding-agent-id-hint"
          />
          <p id="onboarding-agent-id-hint" className="text-xs text-muted-foreground">
            A short id for the AI app you&apos;re protecting — letters, numbers, dashes. It labels
            every decision on your dashboard.
          </p>
        </div>
        <div>
          <Button type="submit" disabled={submitting || agentId.trim() === ''}>
            {submitting ? 'Creating key…' : 'Create my API key'}
          </Button>
        </div>
      </form>
    );
  }

  return (
    <div className="grid gap-6">
      <div className="grid gap-2">
        <Label htmlFor="onboarding-created-key">
          Your secret key — copy it now, it&apos;s shown only once
        </Label>
        <div className="flex min-w-0 gap-2">
          <Input
            id="onboarding-created-key"
            readOnly
            value={created.plaintext_key}
            onFocus={(event) => event.currentTarget.select()}
            className="min-w-0 font-mono text-xs"
          />
          <Button
            type="button"
            variant={copied ? 'outline' : 'default'}
            onClick={copyKey}
            aria-label="Copy your secret key"
          >
            {copied ? <IconCheck aria-hidden /> : <IconCopy aria-hidden />}
            {copied ? 'Copied' : 'Copy'}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          Treat it like a password. Set it as{' '}
          <span className="font-mono text-foreground">TLG_API_KEY</span> in your app&apos;s
          environment — the snippets below expect it there.
        </p>
      </div>

      <CopyBlock
        label="Option 1 · Add the SDK yourself"
        content={buildSdkSnippet({ baseUrl, agentId: cleanAgentId })}
      />
      <div className="grid gap-2">
        <Label id="onboarding-assistant-label">Pick your AI coding assistant</Label>
        <div
          className="flex flex-wrap gap-2"
          role="group"
          aria-labelledby="onboarding-assistant-label"
        >
          {assistantOptions.map((option) => (
            <Button
              key={option.id}
              type="button"
              size="sm"
              variant={assistant === option.id ? 'default' : 'outline'}
              aria-pressed={assistant === option.id}
              onClick={() => setAssistant(option.id)}
            >
              {option.label}
            </Button>
          ))}
        </div>
      </div>
      <CopyBlock
        label={`Option 2 · Paste this into ${selectedAssistant?.label ?? 'your AI coding assistant'}`}
        content={buildAssistantPrompt({ baseUrl, agentId: cleanAgentId, assistant })}
      />

      <div className="flex flex-wrap items-center gap-2">
        <Button asChild>
          <Link href={`/onboarding/verify${contextQuery}`}>
            I&apos;ve added it — watch for my first event
            <IconArrowRight aria-hidden />
          </Link>
        </Button>
        <Button asChild variant="ghost">
          <Link href={`/${contextQuery}`}>Skip setup</Link>
        </Button>
      </div>
    </div>
  );
}
