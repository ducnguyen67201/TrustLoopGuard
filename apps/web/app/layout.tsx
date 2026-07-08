import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { IBM_Plex_Mono, Inter } from 'next/font/google';
import { Toaster } from '@/components/ui/sonner';
import { VersionWatcher } from '@/components/version-watcher';
import { ThemeProvider } from '@/components/theme-provider';
import './globals.css';

// Two-face system: Inter carries UI + prose, IBM Plex Mono carries data/code.
const inter = Inter({
  display: 'swap',
  subsets: ['latin'],
  variable: '--font-inter',
});

const ibmPlexMono = IBM_Plex_Mono({
  display: 'swap',
  subsets: ['latin'],
  variable: '--font-ibm-plex-mono',
  weight: ['400', '500', '600', '700'],
});

export const metadata: Metadata = {
  title: 'TrustLoopGuard',
  description:
    'TrustLoopGuard dashboard. Overview of guardrail decisions and policy activity returned by tl-server.',
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
    <html
      lang="en"
      className={`${inter.variable} ${ibmPlexMono.variable}`}
      suppressHydrationWarning
    >
      <body className="bg-background text-foreground antialiased">
        <ThemeProvider
          attribute="class"
          defaultTheme="dark"
          enableSystem
          disableTransitionOnChange
        >
          {children}
          <Toaster />
          <VersionWatcher />
        </ThemeProvider>
      </body>
    </html>
  );
}
