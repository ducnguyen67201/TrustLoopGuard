'use client';

import { ShieldCheck, Swords, Unplug, Zap } from 'lucide-react';
import type { ReactNode } from 'react';
import { useMemo, useState } from 'react';

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
    <main className="min-h-screen bg-background text-foreground">
      <section className="border-b px-4 py-8 lg:px-8">
        <div className="mx-auto flex max-w-7xl flex-col gap-5">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="outline" className="rounded-sm">
              Open demo
            </Badge>
            <Badge variant="secondary" className="rounded-sm">
              Chat surface
            </Badge>
          </div>
          <div className="grid gap-4 lg:grid-cols-[1fr_360px] lg:items-end">
            <div className="grid gap-3">
              <h1 className="text-3xl font-semibold tracking-normal md:text-4xl">
                Agent Breakaway Arena
              </h1>
              <p className="max-w-3xl text-sm leading-6 text-muted-foreground md:text-base">
                Connect a raw agent and a TrustLoopGuard-protected agent. The arena sends the same
                breaker prompts to both adapters and compares what leaked against what was blocked.
              </p>
            </div>
            <div className="grid grid-cols-3 gap-2">
              <Metric label="Cases" value={breakCases.length || '-'} />
              <Metric label="Raw fail" value={score.rawFailed} />
              <Metric label="Guard pass" value={score.guardedPassed} />
            </div>
          </div>
        </div>
      </section>

      <section className="mx-auto grid max-w-7xl gap-4 px-4 py-6 lg:grid-cols-[380px_1fr] lg:px-8">
        <div className="grid gap-4 self-start">
          <AdapterCard
            title="Raw agent"
            icon={<Unplug className="size-4" />}
            state={raw}
            onUrlChange={(url) => setRaw({ url, profile: null, error: null })}
          />
          <AdapterCard
            title="Guarded agent"
            icon={<ShieldCheck className="size-4" />}
            state={guarded}
            onUrlChange={(url) => setGuarded({ url, profile: null, error: null })}
          />
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => void connectAdapters()} disabled={!canRun} variant="outline">
              Connect
            </Button>
            <Button onClick={() => void runArena()} disabled={!canRun}>
              <Swords className="size-4" />
              Run breaker
            </Button>
          </div>
          {runError ? <p className="text-sm text-destructive">{runError}</p> : null}
        </div>

        <div className="grid gap-4">
          <Card className="rounded-md">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Zap className="size-4" />
                Results
              </CardTitle>
              <CardDescription>
                The arena calls each adapter directly from your browser. Connected agents must allow
                this web origin through CORS.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {results.length === 0 ? (
                <EmptyResults state={runState} />
              ) : (
                <ResultsTable results={results} />
              )}
            </CardContent>
          </Card>
        </div>
      </section>
    </main>
  );
}

function AdapterCard({
  title,
  icon,
  state,
  onUrlChange,
}: {
  title: string;
  icon: ReactNode;
  state: AdapterState;
  onUrlChange: (url: string) => void;
}) {
  return (
    <Card className="rounded-md">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          {icon}
          {title}
        </CardTitle>
        <CardDescription>Base URL for an adapter that exposes `/arena/*`.</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3">
        <div className="grid gap-2">
          <Label htmlFor={`${title}-url`}>Agent URL</Label>
          <Input
            id={`${title}-url`}
            value={state.url}
            onChange={(event) => onUrlChange(event.target.value)}
            placeholder="http://127.0.0.1:8788"
          />
        </div>
        {state.profile ? (
          <div className="rounded-md border bg-muted/30 p-3 text-sm">
            <div className="font-medium">{state.profile.displayName}</div>
            <div className="mt-1 text-muted-foreground">
              Protected: {state.profile.protectedInformationName}
            </div>
          </div>
        ) : null}
        {state.error ? <p className="text-sm text-destructive">{state.error}</p> : null}
      </CardContent>
    </Card>
  );
}

function ResultsTable({ results }: { results: readonly ArenaCaseResult[] }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[760px] text-sm">
        <thead>
          <tr className="border-b text-left text-muted-foreground">
            <th className="py-2 pr-4 font-medium">Attack</th>
            <th className="py-2 pr-4 font-medium">Raw agent</th>
            <th className="py-2 pr-4 font-medium">Guarded agent</th>
            <th className="py-2 font-medium">Trace</th>
          </tr>
        </thead>
        <tbody>
          {results.map((result) => (
            <tr key={result.breakCase.label} className="border-b last:border-0">
              <td className="max-w-[260px] py-3 pr-4 align-top">
                <div className="font-medium">{result.breakCase.label}</div>
                <div className="mt-1 text-xs leading-5 text-muted-foreground">
                  {result.breakCase.userMessage}
                </div>
              </td>
              <ResultCell result={result.raw} />
              <ResultCell result={result.guarded} />
              <td className="py-3 align-top font-mono text-xs text-muted-foreground">
                {result.guarded.response?.traceId ?? '-'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function ResultCell({ result }: { result: AdapterRunResult }) {
  return (
    <td className="max-w-[260px] py-3 pr-4 align-top">
      <Badge variant="outline" className={scoreClassName(result.score.status)}>
        {result.score.label}
      </Badge>
      <div className="mt-2 line-clamp-3 text-xs leading-5 text-muted-foreground">
        {result.response?.content ?? result.score.detail}
      </div>
    </td>
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
    <div className="flex min-h-[300px] items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
      {message}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="rounded-md border bg-card px-3 py-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 text-xl font-semibold">{value}</div>
    </div>
  );
}

async function loadProfile(url: string): Promise<{ data: ArenaAgentProfile | null; error: string | null }> {
  try {
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

function normalizeUrl(url: string): string {
  return url.trim().replace(/\/$/, '');
}

function scoreClassName(status: ArenaScoredResult['status']): string {
  if (status === 'pass') return 'rounded-sm border-emerald-300 bg-emerald-50 text-emerald-700';
  if (status === 'fail') return 'rounded-sm border-rose-300 bg-rose-50 text-rose-700';
  return 'rounded-sm border-amber-300 bg-amber-50 text-amber-700';
}
