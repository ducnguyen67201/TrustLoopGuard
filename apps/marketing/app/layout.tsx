import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { IBM_Plex_Mono } from 'next/font/google';
import './globals.css';

const ibmPlexMono = IBM_Plex_Mono({
  display: 'swap',
  subsets: ['latin'],
  variable: '--font-ibm-plex-mono',
  weight: ['400', '500', '600', '700'],
});

export const metadata: Metadata = {
  title: 'TrustLoopGuard — Real-time guardrails for AI agents',
  description:
    'Catch unsafe agent output before your users see it. TrustLoopGuard returns a safety verdict in milliseconds — allow, rewrite, block, or escalate.',
  metadataBase: new URL('https://trustloopguard.dev'),
  openGraph: {
    title: 'TrustLoopGuard',
    description: 'Real-time guardrails for AI agents. Verdicts in milliseconds.',
    type: 'website',
  },
};

interface RootLayoutProps {
  children: ReactNode;
}

export default function RootLayout({ children }: RootLayoutProps) {
  return (
    <html lang="en" className={ibmPlexMono.variable}>
      <body className="min-h-svh font-sans">{children}</body>
    </html>
  );
}
