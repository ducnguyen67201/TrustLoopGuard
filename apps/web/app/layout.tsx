import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { IBM_Plex_Mono, Instrument_Sans } from 'next/font/google';
import { Toaster } from '@/components/ui/sonner';
import { VersionWatcher } from '@/components/version-watcher';
import { ThemeProvider } from '@/components/theme-provider';
import './globals.css';

const themeInitScript = `
(function () {
  try {
    var root = document.documentElement;
    var storedTheme = window.localStorage.getItem('theme') || 'dark';
    var resolvedTheme = storedTheme === 'system'
      ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
      : storedTheme;
    root.classList.remove('light', 'dark');
    root.classList.add(resolvedTheme === 'light' ? 'light' : 'dark');
    root.style.colorScheme = resolvedTheme === 'light' ? 'light' : 'dark';
  } catch (_) {}
})();
`;

// Instrument Sans carries UI + prose; IBM Plex Mono is reserved for data/code.
const instrumentSans = Instrument_Sans({
  display: 'swap',
  subsets: ['latin'],
  variable: '--font-instrument-sans',
});

const ibmPlexMono = IBM_Plex_Mono({
  display: 'swap',
  subsets: ['latin'],
  variable: '--font-ibm-plex-mono',
  weight: ['400', '500', '600', '700'],
});

export const metadata: Metadata = {
  title: 'Featherlane AI',
  description:
    'Featherlane AI dashboard. Overview of guardrail decisions and policy activity returned by tl-server.',
  icons: {
    icon: [{ url: '/featherlane-ai-logo-dark.png', type: 'image/png' }],
    shortcut: ['/featherlane-ai-logo-dark.png'],
    apple: [{ url: '/featherlane-ai-logo-dark.png', type: 'image/png' }],
  },
};

interface RootLayoutProps {
  children: ReactNode;
}

export default function RootLayout({ children }: RootLayoutProps) {
  return (
    <html
      lang="en"
      className={`${instrumentSans.variable} ${ibmPlexMono.variable} dark`}
      suppressHydrationWarning
    >
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeInitScript }} />
      </head>
      <body className="bg-background font-sans text-foreground antialiased">
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem disableTransitionOnChange>
          {children}
          <Toaster />
          <VersionWatcher />
        </ThemeProvider>
      </body>
    </html>
  );
}
