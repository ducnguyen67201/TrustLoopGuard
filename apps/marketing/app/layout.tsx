import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { GeistMono } from 'geist/font/mono';
import { GeistSans } from 'geist/font/sans';
import './globals.css';

export const metadata: Metadata = {
  title: 'TrustLoopGuard - Runtime guardrails for AI agents',
  description:
    'TrustLoopGuard checks agent actions before they reach users. Return allow, rewrite, block, or escalate with an auditable trace.',
  metadataBase: new URL('https://trustloopguard.dev'),
  openGraph: {
    title: 'TrustLoopGuard',
    description:
      'Runtime guardrails for AI agents. Check every proposed action and keep an auditable trace.',
    type: 'website',
  },
  icons: {
    icon: [{ url: '/trustloop-logo.svg', type: 'image/svg+xml' }],
    shortcut: ['/trustloop-logo.svg'],
    apple: [{ url: '/trustloop-logo.svg', type: 'image/svg+xml' }],
  },
};

interface RootLayoutProps {
  children: ReactNode;
}

export default function RootLayout({ children }: RootLayoutProps) {
  return (
    <html lang="en" className={`${GeistSans.variable} ${GeistMono.variable}`}>
      <body id="top" className="min-h-svh font-sans">
        {children}
      </body>
    </html>
  );
}
