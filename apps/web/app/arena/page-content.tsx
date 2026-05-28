'use client';

import { Radio, ShieldCheck, Swords, Trophy, Unplug } from 'lucide-react';
import type { ReactNode } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  buildArenaChatBreakCases,
  parseArenaAgentProfile,
  parseArenaChatResponse,
  scoreArenaResponse,
  type ArenaAgentProfile,
  type ArenaBreakCase,
  type ArenaChatResponse,
  type ArenaJsonValue,
  type ArenaScoredResult,
} from '@/lib/arena';
import { cn } from '@/lib/utils';

type AdapterKind = 'raw' | 'guarded';
type RunState = 'idle' | 'connecting' | 'running' | 'complete' | 'error';

interface AdapterState {
  url: string;
  profile: ArenaAgentProfile | null;
  error: string | null;
}

interface AdapterRunResult {
  response: ArenaChatResponse | null;
  score: ArenaScoredResult;
}

interface ArenaCaseResult {
  breakCase: ArenaBreakCase;
  raw: AdapterRunResult;
  guarded: AdapterRunResult;
}

const defaultRawUrl = 'http://127.0.0.1:8787';
const defaultGuardedUrl = 'http://127.0.0.1:8788';

export function ArenaPageContent() {
  const [raw, setRaw] = useState<AdapterState>({
    url: defaultRawUrl,
    profile: null,
    error: null,
  });
  const [guarded, setGuarded] = useState<AdapterState>({
    url: defaultGuardedUrl,
    profile: null,
    error: null,
  });
  const [results, setResults] = useState<ArenaCaseResult[]>([]);
  const [runState, setRunState] = useState<RunState>('idle');
  const [runError, setRunError] = useState<string | null>(null);

  const targetProfile = guarded.profile ?? raw.profile;
  const breakCases = useMemo(
    () => (targetProfile ? buildArenaChatBreakCases(targetProfile) : []),
    [targetProfile],
  );

  async function connectAdapters(): Promise<void> {
    setRunState('connecting');
    setRunError(null);
    setResults([]);

    const [rawProfile, guardedProfile] = await Promise.all([
      loadProfile(raw.url),
      loadProfile(guarded.url),
    ]);

    setRaw((current) => ({
      ...current,
      profile: rawProfile.data,
      error: rawProfile.error,
    }));
    setGuarded((current) => ({
      ...current,
      profile: guardedProfile.data,
      error: guardedProfile.error,
    }));

    if (rawProfile.error || guardedProfile.error) {
      setRunState('error');
      setRunError('One or more adapters could not be connected.');
      return;
    }

    setRunState('idle');
  }

  async function runArena(): Promise<void> {
    setRunState('running');
    setRunError(null);
    setResults([]);

    const [rawProfileResult, guardedProfileResult] = await Promise.all([
      raw.profile ? Promise.resolve({ data: raw.profile, error: null }) : loadProfile(raw.url),
      guarded.profile
        ? Promise.resolve({ data: guarded.profile, error: null })
        : loadProfile(guarded.url),
    ]);

    setRaw((current) => ({
      ...current,
      profile: rawProfileResult.data,
      error: rawProfileResult.error,
    }));
    setGuarded((current) => ({
      ...current,
      profile: guardedProfileResult.data,
      error: guardedProfileResult.error,
    }));

    if (!rawProfileResult.data || !guardedProfileResult.data) {
      setRunState('error');
      setRunError('Connect both adapters before running the arena.');
      return;
    }

    const cases = buildArenaChatBreakCases(guardedProfileResult.data);
    const nextResults: ArenaCaseResult[] = [];

    for (const breakCase of cases) {
      const [rawResponse, guardedResponse] = await Promise.all([
        sendChat(raw.url, breakCase.userMessage),
        sendChat(guarded.url, breakCase.userMessage),
      ]);

      nextResults.push({
        breakCase,
        raw: {
          response: rawResponse.data,
          score: rawResponse.error
            ? adapterErrorScore(rawResponse.error)
            : scoreArenaResponse(breakCase, rawResponse.data, 'raw'),
        },
        guarded: {
          response: guardedResponse.data,
          score: guardedResponse.error
            ? adapterErrorScore(guardedResponse.error)
            : scoreArenaResponse(breakCase, guardedResponse.data, 'guarded'),
        },
      });

      setResults([...nextResults]);
    }

    setRunState('complete');
  }

  const score = scoreSummary(results);
  const canRun = runState !== 'connecting' && runState !== 'running';

  return (
    <main className="flex h-screen overflow-hidden bg-[#f4f1ea] text-[#171512]">
      <div className="flex min-h-0 w-full flex-col">
        <section className="shrink-0 border-b border-[#211f1a] bg-[#171512] px-4 py-4 text-[#f8f4e8] lg:px-8">
          <div className="mx-auto grid max-w-7xl gap-4 lg:grid-cols-[1fr_360px] lg:items-end">
            <div className="grid gap-3">
              <div className="flex flex-wrap items-center gap-2">
                <Badge className="rounded-sm border-[#ff6900] bg-[#ff6900] text-black">
                  Open demo
                </Badge>
                <Badge className="rounded-sm border-[#39342b] bg-[#25221d] text-[#d8cfbd]">
                  Chat surface
                </Badge>
                <Badge className="rounded-sm border-[#39342b] bg-[#25221d] text-[#d8cfbd]">
                  Live match
                </Badge>
              </div>
              <div className="grid gap-2">
                <p className="text-xs font-semibold tracking-[0.24em] text-[#ffb36b] uppercase">
                  Agent Breakaway Arena
                </p>
                <h1 className="max-w-4xl text-3xl leading-tight font-semibold tracking-normal md:text-4xl">
                  Raw agent vs guarded agent
                </h1>
                <p className="max-w-3xl text-sm leading-6 text-[#c9c0b0]">
                  Run the same breaker prompts against two adapters and watch the scoreboard
                  separate leaks from TrustLoopGuard blocks.
                </p>
              </div>
            </div>
            <div className="grid grid-cols-3 gap-2 rounded-md border border-[#39342b] bg-[#0f0e0c] p-2 shadow-[0_16px_44px_rgba(0,0,0,0.28)]">
              <Metric label="Cases" value={breakCases.length || '-'} tone="neutral" />
              <Metric label="Raw fail" value={score.rawFailed} tone="danger" />
              <Metric label="Guard pass" value={score.guardedPassed} tone="safe" />
            </div>
          </div>
        </section>

        <section className="mx-auto grid min-h-0 w-full max-w-7xl flex-1 gap-4 px-4 py-4 lg:grid-cols-[360px_1fr] lg:px-8">
          <div className="grid min-h-0 content-start gap-3 overflow-y-auto pr-1 pb-1">
            <div className="rounded-md border border-[#211f1a] bg-[#fffaf0] p-3 shadow-[4px_4px_0_#211f1a]">
              <div className="mb-3 flex items-center justify-between gap-3">
                <div className="flex items-center gap-2 text-sm font-semibold">
                  <Radio className="size-4 text-[#ff6900]" />
                  Contenders
                </div>
                <Badge variant="outline" className={arenaStateClassName(runState)}>
                  {runState}
                </Badge>
              </div>
              <div className="grid gap-3">
                <ArenaLane
                  label="Raw"
                  value={raw.profile ? 'online' : raw.error ? 'error' : 'idle'}
                />
                <ArenaLane
                  label="Guarded"
                  value={guarded.profile ? 'online' : guarded.error ? 'error' : 'idle'}
                />
              </div>
            </div>
            <AdapterCard
              title="Raw agent"
              icon={<Unplug className="size-4" />}
              state={raw}
              tone="raw"
              onUrlChange={(url) => setRaw({ url, profile: null, error: null })}
            />
            <AdapterCard
              title="Guarded agent"
              icon={<ShieldCheck className="size-4" />}
              state={guarded}
              tone="guarded"
              onUrlChange={(url) => setGuarded({ url, profile: null, error: null })}
            />
            <div className="flex flex-wrap gap-2 rounded-md border border-[#211f1a] bg-[#171512] p-2 shadow-[4px_4px_0_#211f1a]">
              <Button
                onClick={() => void connectAdapters()}
                disabled={!canRun}
                variant="outline"
                className="border-[#f8f4e8] bg-[#f8f4e8] text-[#171512] hover:bg-white"
              >
                Connect
              </Button>
              <Button
                onClick={() => void runArena()}
                disabled={!canRun}
                className="bg-[#ff6900] text-black hover:bg-[#ff7d24]"
              >
                <Swords className="size-4" />
                Run breaker
              </Button>
            </div>
            {runError ? <p className="text-sm text-destructive">{runError}</p> : null}
          </div>

          <div className="min-h-0">
            <Card className="grid h-full min-h-0 grid-rows-[auto_1fr] overflow-hidden rounded-md border-[#211f1a] bg-[#fffaf0] shadow-[4px_4px_0_#211f1a]">
              <CardHeader className="shrink-0 border-b border-[#211f1a] bg-[#171512] py-4 text-[#f8f4e8]">
                <CardTitle className="flex items-center gap-2 text-base">
                  <Trophy className="size-4 text-[#ff6900]" />
                  Match transcript
                </CardTitle>
                <CardDescription className="text-[#c9c0b0]">
                  Each breaker turn appears as a chat exchange. The transcript scrolls inside this
                  panel while the arena controls stay fixed.
                </CardDescription>
              </CardHeader>
              <CardContent className="min-h-0 p-0">
                {results.length === 0 ? (
                  <EmptyResults state={runState} />
                ) : (
                  <MatchFeed results={results} />
                )}
              </CardContent>
            </Card>
          </div>
        </section>
      </div>
    </main>
  );
}

function AdapterCard({
  title,
  icon,
  state,
  tone,
  onUrlChange,
}: {
  title: string;
  icon: ReactNode;
  state: AdapterState;
  tone: AdapterKind;
  onUrlChange: (url: string) => void;
}) {
  const isGuarded = tone === 'guarded';

  return (
    <Card
      className={cn(
        'overflow-hidden rounded-md border-[#211f1a] bg-[#fffaf0] shadow-[4px_4px_0_#211f1a]',
        isGuarded ? 'border-l-4 border-l-[#13915d]' : 'border-l-4 border-l-[#d9442f]',
      )}
    >
      <CardHeader className="pb-3">
        <CardTitle className="flex items-center gap-2 text-base">
          <span
            className={cn(
              'flex size-7 items-center justify-center rounded-sm border',
              isGuarded
                ? 'border-[#13915d] bg-[#dbf7e7] text-[#0f6f48]'
                : 'border-[#d9442f] bg-[#ffe3dc] text-[#a72c1d]',
            )}
          >
            {icon}
          </span>
          {title}
        </CardTitle>
        <CardDescription className="text-[#6f675b]">
          Base URL for an adapter that exposes `/arena/*`.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3">
        <div className="grid gap-2">
          <Label htmlFor={`${title}-url`} className="text-xs font-semibold uppercase">
            Agent URL
          </Label>
          <Input
            id={`${title}-url`}
            value={state.url}
            onChange={(event) => onUrlChange(event.target.value)}
            placeholder="http://127.0.0.1:8788"
            className="h-10 border-[#211f1a] bg-white font-mono text-sm shadow-none focus-visible:ring-[#ff6900]"
          />
        </div>
        {state.profile ? (
          <div className="rounded-md border border-[#d8cfbd] bg-[#f8f4e8] p-3 text-sm">
            <div className="font-medium">{state.profile.displayName}</div>
            <div className="mt-1 text-[#6f675b]">
              Protected: {state.profile.protectedInformationName}
            </div>
          </div>
        ) : null}
        {state.error ? <p className="text-sm text-destructive">{state.error}</p> : null}
      </CardContent>
    </Card>
  );
}

function MatchFeed({ results }: { results: readonly ArenaCaseResult[] }) {
  const bottomRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: 'end', behavior: 'smooth' });
  }, [results.length]);

  return (
    <div className="h-full overflow-y-auto bg-[#fffaf0] p-4">
      <div className="grid gap-5">
        {results.map((result, index) => (
          <section key={result.breakCase.label} className="grid gap-3">
            <div className="flex justify-center">
              <div className="rounded-sm border border-[#d8cfbd] bg-[#efe7d8] px-3 py-1 font-mono text-[11px] text-[#6f675b] uppercase">
                Turn {index + 1} / {result.breakCase.label}
              </div>
            </div>
            <div className="max-w-[78%] rounded-md border border-[#211f1a] bg-white px-4 py-3 shadow-[3px_3px_0_#211f1a]">
              <div className="mb-1 font-mono text-[11px] text-[#6f675b] uppercase">Breaker</div>
              <div className="text-sm leading-6">{result.breakCase.userMessage}</div>
            </div>
            <div className="grid gap-3 lg:grid-cols-2">
              <ResultBubble label="Raw agent" tone="raw" result={result.raw} />
              <ResultBubble label="Guarded agent" tone="guarded" result={result.guarded} />
            </div>
          </section>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}

function ResultBubble({
  label,
  tone,
  result,
}: {
  label: string;
  tone: AdapterKind;
  result: AdapterRunResult;
}) {
  const response = result.response;
  const isGuarded = tone === 'guarded';

  return (
    <div
      className={cn(
        'rounded-md border bg-[#f8f4e8] px-4 py-3',
        isGuarded ? 'border-[#13915d]' : 'border-[#d9442f]',
      )}
    >
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <div className="font-mono text-[11px] text-[#6f675b] uppercase">{label}</div>
        <Badge variant="outline" className={scoreClassName(result.score.status)}>
          {result.score.label}
        </Badge>
      </div>
      <div className="text-sm leading-6 text-[#171512]">
        {response?.content ?? result.score.detail}
      </div>
      {response ? (
        <div className="mt-3 grid gap-1 rounded-sm border border-[#d8cfbd] bg-[#fffaf0] p-2 font-mono text-[11px] leading-5 text-[#4f493f]">
          <div>agent.finish_reason = {response.finishReason}</div>
          <div>tlg.verdict_header = {response.verdict ?? 'none'}</div>
          <div>tlg.phase_header = {response.phase ?? 'none'}</div>
          {response.traceId ? (
            <div className="break-all">tlg.trace_id = {response.traceId}</div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function EmptyResults({ state }: { state: RunState }) {
  const message =
    state === 'connecting'
      ? 'Connecting adapters...'
      : state === 'running'
        ? 'Running breaker prompts...'
        : 'Connect both agents, then run the breaker.';

  return (
    <div className="flex h-full min-h-[360px] items-center justify-center bg-[#fffaf0] p-6">
      <div className="grid w-full max-w-xl gap-5">
        <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3">
          <div className="h-2 rounded-sm border border-[#d9442f] bg-[#ffe3dc]" />
          <Swords className="size-5 text-[#ff6900]" />
          <div className="h-2 rounded-sm border border-[#13915d] bg-[#dbf7e7]" />
        </div>
        <div className="rounded-md border border-dashed border-[#8f8576] bg-[#f8f4e8] px-4 py-8 text-center text-sm text-[#6f675b]">
          {message}
        </div>
      </div>
    </div>
  );
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string;
  value: number | string;
  tone: 'neutral' | 'danger' | 'safe';
}) {
  return (
    <div className="rounded-sm border border-[#39342b] bg-[#191713] px-3 py-3">
      <div className="text-xs text-[#9f9585]">{label}</div>
      <div
        className={cn(
          'mt-1 text-2xl font-semibold tabular-nums',
          tone === 'danger' && 'text-[#ff7a66]',
          tone === 'safe' && 'text-[#70e0a2]',
          tone === 'neutral' && 'text-[#f8f4e8]',
        )}
      >
        {value}
      </div>
    </div>
  );
}

async function loadProfile(
  url: string,
): Promise<{ data: ArenaAgentProfile | null; error: string | null }> {
  try {
    // Keep arena adapter calls browser-direct so user-provided URLs do not become a server-side proxy surface.
    const response = await fetch(`${normalizeUrl(url)}/arena/profile`);
    const body = await readJson(response);
    if (!response.ok) return { data: null, error: `HTTP ${response.status}` };
    return { data: parseArenaAgentProfile(body), error: null };
  } catch (error) {
    return { data: null, error: error instanceof Error ? error.message : String(error) };
  }
}

async function sendChat(
  url: string,
  message: string,
): Promise<{ data: ArenaChatResponse | null; error: string | null }> {
  try {
    // Keep arena adapter calls browser-direct so user-provided URLs do not become a server-side proxy surface.
    const response = await fetch(`${normalizeUrl(url)}/arena/chat`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ message }),
    });
    const body = await readJson(response);
    if (!response.ok) return { data: null, error: `HTTP ${response.status}` };
    return { data: parseArenaChatResponse(body), error: null };
  } catch (error) {
    return { data: null, error: error instanceof Error ? error.message : String(error) };
  }
}

async function readJson(response: Response): Promise<ArenaJsonValue> {
  const text = await response.text();
  return text ? (JSON.parse(text) as ArenaJsonValue) : {};
}

function adapterErrorScore(error: string): ArenaScoredResult {
  return { status: 'error', label: 'Error', detail: error };
}

function scoreSummary(results: readonly ArenaCaseResult[]) {
  return {
    rawFailed: results.filter((result) => result.raw.score.status === 'fail').length,
    guardedPassed: results.filter((result) => result.guarded.score.status === 'pass').length,
  };
}

function ArenaLane({ label, value }: { label: string; value: 'idle' | 'online' | 'error' }) {
  return (
    <div className="grid grid-cols-[72px_1fr_64px] items-center gap-3 text-xs">
      <div className="font-semibold">{label}</div>
      <div className="h-2 overflow-hidden rounded-sm border border-[#211f1a] bg-[#e7ddcc]">
        <div
          className={cn(
            'h-full',
            value === 'online' && 'bg-[#13915d]',
            value === 'error' && 'bg-[#d9442f]',
            value === 'idle' && 'bg-[#9f9585]',
          )}
          style={{ width: value === 'idle' ? '34%' : '100%' }}
        />
      </div>
      <div className="font-mono text-[11px] text-[#6f675b] uppercase">{value}</div>
    </div>
  );
}

function arenaStateClassName(state: RunState): string {
  if (state === 'complete') return 'rounded-sm border-[#13915d] bg-[#dbf7e7] text-[#0f6f48]';
  if (state === 'error') return 'rounded-sm border-[#d9442f] bg-[#ffe3dc] text-[#a72c1d]';
  if (state === 'running' || state === 'connecting') {
    return 'rounded-sm border-[#ff6900] bg-[#fff0df] text-[#9b4700]';
  }
  return 'rounded-sm border-[#d8cfbd] bg-[#f8f4e8] text-[#6f675b]';
}

function normalizeUrl(url: string): string {
  return url.trim().replace(/\/$/, '');
}

function scoreClassName(status: ArenaScoredResult['status']): string {
  if (status === 'pass') return 'rounded-sm border-[#13915d] bg-[#dbf7e7] text-[#0f6f48]';
  if (status === 'fail') return 'rounded-sm border-[#d9442f] bg-[#ffe3dc] text-[#a72c1d]';
  return 'rounded-sm border-[#ff6900] bg-[#fff0df] text-[#9b4700]';
}
