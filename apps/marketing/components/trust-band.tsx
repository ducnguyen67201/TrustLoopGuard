import type { ReactNode } from 'react';

interface TrustItem {
  label: ReactNode;
  icon: ReactNode;
}

function IconSigned() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden="true"
    >
      <path d="M9 12l2 2 4-4" strokeLinecap="round" strokeLinejoin="round" />
      <circle cx="12" cy="12" r="9" />
    </svg>
  );
}

function IconLedger() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden="true"
    >
      <rect x="4" y="3" width="16" height="18" rx="1.5" />
      <path d="M8 8h8M8 12h8M8 16h5" strokeLinecap="round" />
    </svg>
  );
}

function IconShield() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden="true"
    >
      <path d="M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6z" strokeLinejoin="round" />
    </svg>
  );
}

function IconBolt() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden="true"
    >
      <path d="M13 3L5 14h6l-1 7 8-11h-6z" strokeLinejoin="round" />
    </svg>
  );
}

function IconServer() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden="true"
    >
      <rect x="3" y="4" width="18" height="7" rx="1.5" />
      <rect x="3" y="13" width="18" height="7" rx="1.5" />
      <path d="M7 7.5h.01M7 16.5h.01" strokeLinecap="round" />
    </svg>
  );
}

function IconChainOff() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden="true"
    >
      <path d="M9 12h6" strokeLinecap="round" />
      <path d="M8 8.5A3.5 3.5 0 005 12a3.5 3.5 0 003.5 3.5" strokeLinecap="round" />
      <path d="M16 8.5A3.5 3.5 0 0119 12a3.5 3.5 0 01-3.5 3.5" strokeLinecap="round" />
      <path d="M4 4l16 16" strokeLinecap="round" />
    </svg>
  );
}

const ITEMS: TrustItem[] = [
  { icon: <IconSigned />, label: 'Signed decisions · ed25519' },
  { icon: <IconLedger />, label: 'Tamper-evident audit log' },
  { icon: <IconShield />, label: 'SOC 2 · in progress' },
  { icon: <IconBolt />, label: 'Sub-10ms p99 check' },
  { icon: <IconServer />, label: 'Self-host · your VPC' },
  { icon: <IconChainOff />, label: 'No blockchain · real rails' },
];

export function TrustBand() {
  return (
    <section aria-label="Security and trust guarantees" className="trust-band">
      <ul className="trust-inner">
        {ITEMS.map((item, i) => (
          <li key={i} className="trust-cell">
            {item.icon}
            <span>{item.label}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
