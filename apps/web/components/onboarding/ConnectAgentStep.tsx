'use client';

import {
  IconArrowRight,
  IconCheck,
  IconCopy,
  IconKey,
  IconTerminal2,
  IconSparkles,
  IconShieldBolt,
} from '@tabler/icons-react';
import Link from 'next/link';
import { useState, type FormEvent, type ReactNode } from 'react';
import { toast } from 'sonner';

import { CopyBlock } from '@/components/onboarding/CopyBlock';
import { useFirstTrace, effectVariant } from '@/components/onboarding/useFirstTrace';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { http } from '@/lib/http';
import {
  assistantOptions,
  buildAssistantPrompt,
  buildClaudeCodeHookPrompt,
  buildPaymentSdkSnippet,
  buildSdkSnippet,
  createApiKeyResponseSchema,
  sanitizeAgentId,
  type AssistantKind,
  type CreatedApiKey,
} from '@/lib/onboarding';

// Visible previews stay short (spec: ~4–6 lines) while CopyBlock still copies
// the full snippet. Tuned per surface so each preview ends on a meaningful line.
const SDK_PREVIEW_LINES = 6;
const PROMPT_PREVIEW_LINES = 5;
const HOOK_PREVIEW_LINES = 5;

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
 * the integration paths — SDK quick-start, a paste-into-your-AI-assistant
 * prompt, and a Claude-Code-native PreToolUse hook. The plaintext key lives
 * only in component state; it is never placed in URLs, storage, or snippets.
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
  const cleanAgentId = sanitizeAgentId(agentId).replace(/^-+|-+$/g, '') || defaultAgentId;
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
    // Pre-key state mirrors the post-key two-column grid (same 22rem/1fr
    // track) so revealing the key feels like the right rail resolving into a
    // secret — not a jarring width jump from a skinny page.
    return (
      <div className="grid items-start gap-8 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)] lg:gap-10">
        <aside className="grid gap-5 lg:sticky lg:top-10">
          <form onSubmit={onSubmit} className="grid gap-6 rounded-xl border bg-card p-6 shadow-sm">
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
              <p id="onboarding-agent-id-hint" className="text-xs leading-5 text-muted-foreground">
                A short id for the AI app you&apos;re protecting — letters, numbers, dashes. It
                labels every decision on your dashboard.
              </p>
            </div>
            <Button type="submit" disabled={submitting || agentId.trim() === ''}>
              <IconKey aria-hidden />
              {submitting ? 'Creating key…' : 'Create my API key'}
            </Button>
          </form>
        </aside>

        <FlowPreview />
      </div>
    );
  }

  return (
    <div className="grid items-start gap-8 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)] lg:gap-10">
      {/* Left rail: the key reveal is the single peak; a quiet checklist trails it. */}
      <aside className="grid gap-4 lg:sticky lg:top-10">
        <KeyReveal
          plaintextKey={created.plaintext_key}
          prefix={created.api_key.prefix}
          agentId={cleanAgentId}
          copied={copied}
          onCopy={copyKey}
        />
        <NextSteps />
      </aside>

      {/* Right: one compact, tabbed integration surface. */}
      <div className="grid min-w-0 gap-4">
        <div className="grid gap-1">
          <h2 className="text-base font-semibold tracking-tight">Wire in the guard</h2>
          <p className="text-sm leading-6 text-muted-foreground">
            Pick the path that matches how you build. Every snippet reads{' '}
            <span className="font-mono text-foreground">TLG_API_KEY</span> from the environment —
            preview shows the shape, the copy button takes the whole thing.
          </p>
        </div>

        <Tabs defaultValue="sdk" className="min-w-0 gap-4">
          <TabsList className="w-full">
            <TabsTrigger value="sdk">
              <IconTerminal2 aria-hidden />
              SDK
            </TabsTrigger>
            <TabsTrigger value="assistant">
              <IconSparkles aria-hidden />
              AI assistant
            </TabsTrigger>
            <TabsTrigger value="payments">
              <IconShieldBolt aria-hidden />
              Agent payments
            </TabsTrigger>
            <TabsTrigger value="claude-code">
              <IconShieldBolt aria-hidden />
              Guard Claude Code
            </TabsTrigger>
          </TabsList>

          <TabsContent value="sdk" className="grid gap-3">
            <SurfaceIntro>
              Add TrustLoopGuard by hand: wrap your agent&apos;s model call with{' '}
              <span className="font-mono text-foreground">guard()</span> inside a run.
            </SurfaceIntro>
            <CopyBlock
              label="Add the SDK yourself"
              content={buildSdkSnippet({ baseUrl, agentId: cleanAgentId })}
              previewLines={SDK_PREVIEW_LINES}
            />
          </TabsContent>

          <TabsContent value="payments" className="grid gap-3">
            <SurfaceIntro>
              For ecommerce agents, place TrustLoopGuard between the merchant&apos;s 402 response
              and the wallet signature. A reusable grant proves delegated authority; policies
              enforce standing limits through the same authorization flow.
            </SurfaceIntro>
            <CopyBlock
              label="Authorize x402 payment before signing"
              content={buildPaymentSdkSnippet({ baseUrl, agentId: cleanAgentId })}
              previewLines={SDK_PREVIEW_LINES}
            />
          </TabsContent>

          <TabsContent value="assistant" className="grid gap-4">
            <SurfaceIntro>
              Let your coding assistant do the wiring. Pick yours, then paste the prompt — it
              installs the SDK and wraps your agent for you.
            </SurfaceIntro>
            <div className="grid gap-2">
              <Label
                id="onboarding-assistant-label"
                className="text-3xs font-medium uppercase tracking-label text-muted-foreground"
              >
                Your coding assistant
              </Label>
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
              label={`Paste this into ${selectedAssistant?.label ?? 'your AI coding assistant'}`}
              content={buildAssistantPrompt({ baseUrl, agentId: cleanAgentId, assistant })}
              previewLines={PROMPT_PREVIEW_LINES}
            />
          </TabsContent>

          <TabsContent value="claude-code" className="grid gap-3">
            <SurfaceIntro>
              Claude Code <em className="text-foreground not-italic">is</em> your agent. This
              installs a PreToolUse hook, so every tool call is checked before it runs — no codebase
              to change.
            </SurfaceIntro>
            <CopyBlock
              label="Paste into Claude Code to guard it directly"
              content={buildClaudeCodeHookPrompt({ baseUrl, agentId: cleanAgentId })}
              previewLines={HOOK_PREVIEW_LINES}
            />
          </TabsContent>
        </Tabs>

        <div className="flex items-center justify-between gap-3 rounded-lg border p-3">
          <div>
            <p className="text-sm font-medium">Want TrustLoopGuard to open a PR?</p>
            <p className="text-xs text-muted-foreground">
              Connect GitHub from the agent row and review the generated draft PR.
            </p>
          </div>
          <Button asChild variant="outline" size="sm">
            <Link href={`/agents${contextQuery}`}>
              Agents
              <IconArrowRight aria-hidden />
            </Link>
          </Button>
        </div>

        <FirstEventStatus contextQuery={contextQuery} />
      </div>
    </div>
  );
}

/**
 * Live confirmation, right on this page: as soon as the key exists we listen
 * for the workspace's first guarded request and flip from "listening" to
 * "connected" the moment it lands — the user never has to leave to find out
 * whether their paste worked. The verify page shows the same event in detail.
 */
function FirstEventStatus({ contextQuery }: { contextQuery: string }) {
  const { trace } = useFirstTrace();
  const connected = trace !== null;

  return (
    <div className="grid gap-3 pt-1">
      {connected ? (
        <div
          role="status"
          className="flex flex-wrap items-center gap-2 rounded-lg border border-[var(--badge-allow-border)] bg-[var(--badge-allow-bg)] px-4 py-3 text-sm"
        >
          <IconCheck className="size-4 shrink-0 text-[var(--color-permit)]" aria-hidden />
          <span className="font-medium">You&apos;re connected — we received your request.</span>
          <Badge variant={effectVariant(trace.decision)}>{trace.decision}</Badge>
          <span className="tabular-nums text-muted-foreground">{trace.elapsed_ms}ms</span>
        </div>
      ) : (
        <div
          role="status"
          aria-live="polite"
          className="flex items-center gap-2.5 rounded-lg border bg-muted/40 px-4 py-3 text-sm text-muted-foreground"
        >
          <span aria-hidden className="relative flex size-1.5 shrink-0">
            <span className="absolute inline-flex size-full animate-ping rounded-full bg-primary/60 motion-reduce:hidden" />
            <span className="relative inline-flex size-1.5 rounded-full bg-primary" />
          </span>
          Listening — run your agent once and your first event lands right here.
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <Button asChild>
          <Link href={connected ? `/${contextQuery}` : `/onboarding/verify${contextQuery}`}>
            {connected ? 'Continue to your dashboard' : "I've added it — watch for my first event"}
            <IconArrowRight aria-hidden />
          </Link>
        </Button>
        <Button asChild variant="ghost">
          <Link href={connected ? `/onboarding/verify${contextQuery}` : `/${contextQuery}`}>
            {connected ? 'See the event details' : 'Skip setup'}
          </Link>
        </Button>
      </div>
    </div>
  );
}

/**
 * The one-time secret reveal — the emotional peak of the step. Given real
 * hierarchy: a bordered card with a primary-tinted header, the key on its own
 * row, and a plain-language warning. The secret stays in a selectable Input
 * so it is never re-rendered from a snippet string.
 */
function KeyReveal({
  plaintextKey,
  prefix,
  agentId,
  copied,
  onCopy,
}: {
  plaintextKey: string;
  prefix: string;
  agentId: string;
  copied: boolean;
  onCopy: () => void;
}) {
  return (
    <div className="overflow-hidden rounded-xl border border-primary/30 bg-card shadow-md ring-1 ring-primary/10">
      <div className="flex items-center gap-2 border-b border-primary/20 bg-primary/5 px-4 py-3">
        <span className="flex size-7 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
          <IconKey className="size-4" aria-hidden />
        </span>
        <div className="grid gap-0.5">
          <p className="text-sm font-semibold leading-none">Your secret key</p>
          <p className="text-3xs uppercase tracking-label text-muted-foreground">
            {prefix}··· · shown only once
          </p>
        </div>
      </div>
      <div className="grid gap-3 p-4">
        <Label htmlFor="onboarding-created-key" className="sr-only">
          Your secret API key — copy it now before leaving this page
        </Label>
        <Input
          id="onboarding-created-key"
          readOnly
          value={plaintextKey}
          onFocus={(event) => event.currentTarget.select()}
          className="min-w-0 font-mono text-xs"
        />
        <Button
          type="button"
          className="w-full"
          variant={copied ? 'outline' : 'default'}
          onClick={onCopy}
          aria-label="Copy your secret key"
        >
          {copied ? <IconCheck aria-hidden /> : <IconCopy aria-hidden />}
          {copied ? 'Copied to clipboard' : 'Copy key'}
        </Button>
        <p className="text-xs leading-5 text-muted-foreground">
          Treat it like a password. Set it as{' '}
          <span className="font-mono text-foreground">TLG_API_KEY</span> in your app&apos;s
          environment — you won&apos;t see it again.
        </p>
      </div>
      <p className="border-t border-primary/20 bg-primary/5 px-4 py-2.5 text-3xs font-medium uppercase tracking-label text-muted-foreground">
        Guarding{' '}
        <span className="font-mono normal-case tracking-normal text-foreground">{agentId}</span>
      </p>
    </div>
  );
}

const NEXT_STEPS = [
  'Set the key as TLG_API_KEY',
  'Wire the guard with one of the paths',
  'Run once — watch your first decision land',
] as const;

/**
 * A deliberately quiet trailer under the key card: borderless, muted, small
 * type, no card chrome — so the rail reads as one peak (the key) followed by a
 * subordinate checklist, not two competing bordered boxes.
 */
function NextSteps() {
  return (
    <ol className="grid gap-2.5 px-1">
      {NEXT_STEPS.map((step, index) => (
        <li key={step} className="grid grid-cols-[auto_minmax(0,1fr)] items-start gap-2.5">
          <span className="text-3xs font-medium leading-5 tabular-nums text-muted-foreground/70">
            {index + 1}
          </span>
          <span className="text-xs leading-5 text-muted-foreground">{step}</span>
        </li>
      ))}
    </ol>
  );
}

/**
 * Pre-key right column: a quiet three-beat foreshadow of what the flow
 * produces (key → integrate → first event). Uses the same muted, borderless
 * treatment as the post-key checklist so the right track feels continuous
 * across the reveal instead of swapping layouts.
 */
const FLOW_BEATS = [
  {
    icon: IconKey,
    title: 'Your secret key, once',
    body: 'We create it here and show the plaintext a single time — copy it, set it as TLG_API_KEY.',
  },
  {
    icon: IconTerminal2,
    title: 'Wire in the guard',
    body: 'Pick a path — SDK, an AI-assistant prompt, or a Claude Code hook — and paste it in.',
  },
  {
    icon: IconShieldBolt,
    title: 'Watch the first event',
    body: 'Run your agent once and your first guarded decision lands on the dashboard.',
  },
] as const;

function FlowPreview() {
  return (
    <section aria-label="What happens next" className="grid min-w-0 gap-5">
      <div className="grid gap-1">
        <h2 className="text-base font-semibold tracking-tight">Three steps to a guarded agent</h2>
        <p className="text-sm leading-6 text-muted-foreground">
          Name your agent and we&apos;ll mint its key. Here&apos;s the whole path from here.
        </p>
      </div>
      <ol className="grid gap-0">
        {FLOW_BEATS.map(({ icon: Icon, title, body }, index) => (
          <li
            key={title}
            className="grid grid-cols-[auto_minmax(0,1fr)] gap-4 border-l border-dashed border-border pb-6 pl-0 last:border-l-transparent last:pb-0"
          >
            <span className="relative -left-[0.9375rem] flex size-7 shrink-0 items-center justify-center rounded-full border bg-card text-muted-foreground shadow-sm">
              <Icon className="size-4" aria-hidden />
            </span>
            <div className="-ml-4 grid gap-1">
              <p className="text-sm font-medium leading-none">
                <span className="mr-1.5 text-3xs tabular-nums text-muted-foreground/60">
                  {index + 1}
                </span>
                {title}
              </p>
              <p className="text-sm leading-6 text-muted-foreground">{body}</p>
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}

/** Small intro line above each integration surface. */
function SurfaceIntro({ children }: { children: ReactNode }) {
  return <p className="text-sm leading-6 text-muted-foreground">{children}</p>;
}
