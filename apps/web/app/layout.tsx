import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import './globals.css';

export const metadata: Metadata = {
  title: 'TrustLoopGuard Playground',
  description:
    'Interactive playground for the TrustLoopGuard guardrails server. Submits CheckRequest payloads and renders Decisions returned by tl-server.',
};

interface RootLayoutProps {
  children: ReactNode;
}

export default function RootLayout({ children }: RootLayoutProps) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
