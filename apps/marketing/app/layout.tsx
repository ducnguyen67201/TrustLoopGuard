import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { Inter, Space_Grotesk } from 'next/font/google';
import { GeistMono } from 'geist/font/mono';
import './globals.css';

const inter = Inter({ subsets: ['latin'], variable: '--font-inter', display: 'swap' });
const spaceGrotesk = Space_Grotesk({
  subsets: ['latin'],
  variable: '--font-display',
  display: 'swap',
});

export const metadata: Metadata = {
  title: 'TrustLoopGuard - Spend control & audit for AI agents that move money',
  description:
    'Cap what your AI agent can spend, authorize each payment, refund, or account change before it fires, and prove who did what and why. Traditional rails, no blockchain.',
  metadataBase: new URL('https://trustloopguard.dev'),
  openGraph: {
    title: 'TrustLoopGuard',
    description:
      'Spend caps, per-action authorization, and an audit trail for AI agents that pay, refund, and move money — before it fires.',
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
    <html lang="en" className={`${inter.variable} ${spaceGrotesk.variable} ${GeistMono.variable}`}>
      <body id="top" className="min-h-svh font-sans">
        {children}
      </body>
    </html>
  );
}
