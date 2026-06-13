'use client';

import { ChevronDown, ShieldCheck, Unplug } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

import { RedTeamPanel } from './_components/redteam-panel';

const DEFAULT_RAW_URL = 'http://127.0.0.1:8787';
const DEFAULT_GUARDED_URL = 'http://127.0.0.1:8788';

export function ArenaPageContent() {
  const [rawUrl, setRawUrl] = useState(DEFAULT_RAW_URL);
  const [guardedUrl, setGuardedUrl] = useState(DEFAULT_GUARDED_URL);

  return (
    <div className="mx-auto grid w-full max-w-5xl gap-5 p-4 lg:p-6">
      <div className="grid gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">Arena</h1>
        <p className="text-sm text-muted-foreground">
          Run the same campaign against a raw agent and a TrustLoopGuard-protected agent — the
          before/after. To attack a single agent endpoint, use Attacks.
        </p>
      </div>

      <details className="group rounded-md border">
        <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 text-sm font-medium">
          <span className="flex flex-wrap items-center gap-2">
            <Unplug className="size-4 text-muted-foreground" />
            Targets
            <span className="font-mono text-xs font-normal text-muted-foreground">
              {rawUrl} vs {guardedUrl}
            </span>
          </span>
          <ChevronDown className="size-4 text-muted-foreground transition-transform group-open:rotate-180 motion-reduce:transition-none" />
        </summary>
        <div className="grid gap-4 border-t p-4 md:grid-cols-2">
          <TargetField
            id="raw-url"
            label="Unguarded agent"
            icon={<Unplug className="size-4" />}
            value={rawUrl}
            onChange={setRawUrl}
            placeholder={DEFAULT_RAW_URL}
          />
          <TargetField
            id="guarded-url"
            label="TrustLoopGuard agent"
            icon={<ShieldCheck className="size-4" />}
            value={guardedUrl}
            onChange={setGuardedUrl}
            placeholder={DEFAULT_GUARDED_URL}
          />
        </div>
      </details>

      <RedTeamPanel rawUrl={rawUrl} guardedUrl={guardedUrl} />
    </div>
  );
}

interface TargetFieldProps {
  id: string;
  label: string;
  icon: ReactNode;
  value: string;
  onChange: (next: string) => void;
  placeholder: string;
}

function TargetField({ id, label, icon, value, onChange, placeholder }: TargetFieldProps) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={id} className="flex items-center gap-2 text-xs font-semibold uppercase">
        <span className="flex size-6 items-center justify-center rounded-md border text-muted-foreground">
          {icon}
        </span>
        {label}
      </Label>
      <Input
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="font-mono"
      />
    </div>
  );
}
