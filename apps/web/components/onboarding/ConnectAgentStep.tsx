'use client';

import { IconArrowRight, IconCheck, IconCopy } from '@tabler/icons-react';
import Link from 'next/link';
import { useState, type FormEvent } from 'react';
import { toast } from 'sonner';

import { CopyBlock } from '@/components/onboarding/CopyBlock';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { buildAssistantPrompt, buildSdkSnippet } from '@/lib/onboarding';

type CreateApiKeyResponse = {
  api_key: { id: string; name: string; prefix: string };
  plaintext_key: string;
};

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
}: {
  baseUrl: string;
  environmentId: string;
  defaultAgentId: string;
  workspaceSlug: string;
}) {
  const [agentId, setAgentId] = useState(defaultAgentId);
  const [submitting, setSubmitting] = useState(false);
  const [created, setCreated] = useState<CreateApiKeyResponse | null>(null);
  const [copied, setCopied] = useState(false);

  const workspaceQuery = workspaceSlug ? `?workspace=${encodeURIComponent(workspaceSlug)}` : '';

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    try {
      const res = await fetch(`/api/api-keys${workspaceQuery}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: `${agentId.trim()} key`, environment_id: environmentId }),
      });
      const text = await res.text();
      if (!res.ok) {
        toast.error(
          safeMessage(text) ?? "Couldn't create the key. Please try again.",
        );
        return;
      }
      setCreated(JSON.parse(text) as CreateApiKeyResponse);
    } catch (err) {
      toast.error(
        err instanceof Error
          ? `Couldn't create the key: ${err.message}.`
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

  const cleanAgentId = agentId.trim() === '' ? defaultAgentId : agentId.trim();

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
            onChange={(event) => setAgentId(event.target.value)}
            aria-describedby="onboarding-agent-id-hint"
          />
          <p id="onboarding-agent-id-hint" className="text-xs text-muted-foreground">
            A short id for the AI app you&apos;re protecting. It labels every decision on your
            dashboard.
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
      <CopyBlock
        label="Option 2 · Paste this into your AI coding assistant"
        content={buildAssistantPrompt({ baseUrl, agentId: cleanAgentId })}
      />

      <div className="flex flex-wrap items-center gap-2">
        <Button asChild>
          <Link href={`/onboarding/verify${workspaceQuery}`}>
            I&apos;ve added it — watch for my first event
            <IconArrowRight aria-hidden />
          </Link>
        </Button>
        <Button asChild variant="ghost">
          <Link href={`/${workspaceQuery}`}>Skip setup</Link>
        </Button>
      </div>
    </div>
  );
}

function safeMessage(text: string): string | null {
  try {
    const parsed = JSON.parse(text) as { message?: string; error?: string };
    return parsed.message ?? parsed.error ?? null;
  } catch {
    return text.length > 0 ? text : null;
  }
}
