'use client';

import { useState } from 'react';

const COMMAND = 'npm install @trustloopguard/sdk';

const COPY = {
  en: { label: 'Install the SDK', copy: 'Copy', copied: 'Copied' },
  vi: { label: 'Cài SDK', copy: 'Sao chép', copied: 'Đã sao chép' },
} as const;

export function QuickInstall({ locale }: { locale: keyof typeof COPY }) {
  const copy = COPY[locale];
  const [copied, setCopied] = useState(false);

  async function copyCommand() {
    await navigator.clipboard.writeText(COMMAND);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1_500);
  }

  return (
    <div className="quick-install" aria-label={copy.label}>
      <span>{copy.label}</span>
      <code>{COMMAND}</code>
      <button type="button" onClick={copyCommand} aria-live="polite">
        {copied ? copy.copied : copy.copy}
      </button>
    </div>
  );
}
