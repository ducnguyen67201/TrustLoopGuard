import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { GeistSans } from 'geist/font/sans';
import { GeistMono } from 'geist/font/mono';
import './globals.css';

export const metadata: Metadata = {
  title: 'TrustLoopGuard — Real-time guardrails for AI agents',
  description:
    'Catch unsafe agent output before your users see it. TrustLoopGuard returns a safety verdict in milliseconds — allow, rewrite, block, or escalate.',
  metadataBase: new URL('https://trustloopguard.dev'),
  openGraph: {
    title: 'TrustLoopGuard',
    description:
      'Real-time guardrails for AI agents. Verdicts in milliseconds.',
    type: 'website',
  },
};

interface RootLayoutProps {
  children: ReactNode;
}

export default function RootLayout({ children }: RootLayoutProps) {
  return (
    <html lang="en" className={`${GeistSans.variable} ${GeistMono.variable}`}>
      <body className="min-h-svh font-sans">{children}</body>
    </html>
  );
}
