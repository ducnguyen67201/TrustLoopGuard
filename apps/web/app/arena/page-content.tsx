'use client';

import { ChevronDown, ShieldCheck, Swords, Unplug } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

import { RedTeamPanel } from './_components/redteam-panel';

const DEFAULT_RAW_URL = 'http://127.0.0.1:8787';
const DEFAULT_GUARDED_URL = 'http://127.0.0.1:8788';

export function ArenaPageContent() {
  const [rawUrl, setRawUrl] = useState(DEFAULT_RAW_URL);
  const [guardedUrl, setGuardedUrl] = useState(DEFAULT_GUARDED_URL);

  return (
    <main className="min-h-screen bg-[#f4f1ea] text-[#171512]">
      <header className="border-b border-[#211f1a] bg-[#171512] px-4 py-6 text-[#f8f4e8] lg:px-8">
        <div className="mx-auto grid max-w-5xl gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <Badge className="rounded-sm border-[#ff6900] bg-[#ff6900] text-black">Live demo</Badge>
            <Badge className="rounded-sm border-[#39342b] bg-[#25221d] text-[#d8cfbd]">
              Chat surface
            </Badge>
          </div>
          <div className="grid gap-2">
            <p className="flex items-center gap-2 text-xs font-semibold tracking-[0.24em] text-[#ffb36b] uppercase">
              <Swords className="size-4" />
              Red-Team Arena
            </p>
            <h1 className="max-w-3xl text-3xl leading-tight font-semibold md:text-4xl">
              Watch TrustLoopGuard close the gap
            </h1>
            <p className="max-w-2xl text-sm leading-6 text-[#c9c0b0]">
              An independent attacker runs the same campaign against an unguarded agent and the same
              agent behind TrustLoopGuard. The score is the before/after.
            </p>
          </div>
        </div>
      </header>

      <div className="mx-auto grid max-w-5xl gap-5 px-4 py-6 lg:px-8">
        <details className="group rounded-md border border-[#211f1a] bg-[#fffaf0] shadow-[4px_4px_0_#211f1a]">
          <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 text-sm font-semibold">
            <span className="flex flex-wrap items-center gap-2">
              <Unplug className="size-4 text-[#6f675b]" />
              Targets
              <span className="font-mono text-xs font-normal text-[#6f675b]">
                {rawUrl} vs {guardedUrl}
              </span>
            </span>
            <ChevronDown className="size-4 text-[#6f675b] transition-transform group-open:rotate-180 motion-reduce:transition-none" />
          </summary>
          <div className="grid gap-4 border-t border-[#e7ddcc] p-4 md:grid-cols-2">
            <TargetField
              id="raw-url"
              label="Unguarded agent"
              icon={<Unplug className="size-4" />}
              tone="raw"
              value={rawUrl}
              onChange={setRawUrl}
            />
            <TargetField
              id="guarded-url"
              label="TrustLoopGuard agent"
              icon={<ShieldCheck className="size-4" />}
              tone="guarded"
              value={guardedUrl}
              onChange={setGuardedUrl}
            />
          </div>
        </details>

        <RedTeamPanel rawUrl={rawUrl} guardedUrl={guardedUrl} />
      </div>
    </main>
  );
}

interface TargetFieldProps {
  id: string;
  label: string;
  icon: ReactNode;
  tone: 'raw' | 'guarded';
  value: string;
  onChange: (next: string) => void;
}

function TargetField({ id, label, icon, tone, value, onChange }: TargetFieldProps) {
  const isGuarded = tone === 'guarded';
  return (
    <div className="grid gap-2">
      <Label htmlFor={id} className="flex items-center gap-2 text-xs font-semibold uppercase">
        <span
          className={
            isGuarded
              ? 'flex size-6 items-center justify-center rounded-sm border border-[#13915d] bg-[#dbf7e7] text-[#0f6f48]'
              : 'flex size-6 items-center justify-center rounded-sm border border-[#d9442f] bg-[#ffe3dc] text-[#a72c1d]'
          }
        >
          {icon}
        </span>
        {label}
      </Label>
      <Input
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={isGuarded ? DEFAULT_GUARDED_URL : DEFAULT_RAW_URL}
        className="h-10 border-[#211f1a] bg-white font-mono text-sm shadow-none focus-visible:ring-[#ff6900]"
      />
    </div>
  );
}
