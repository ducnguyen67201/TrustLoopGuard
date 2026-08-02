import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { IBM_Plex_Mono, Inter } from 'next/font/google';
import localFont from 'next/font/local';
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

// Two-face system: Inter carries UI + prose, Departure Mono carries data/code
// (IBM Plex Mono stays as the metric fallback while the pixel face loads).
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

// Departure Mono (SIL OFL) — pixel-grid mono for data/code surfaces.
const departureMono = localFont({
  src: '../public/fonts/DepartureMono-Regular.woff2',
  variable: '--font-pixel',
  display: 'swap',
  weight: '400',
});

export const metadata: Metadata = {
  title: 'Featherlane AI',
  description:
    'Featherlane AI dashboard. Overview of guardrail decisions and policy activity returned by tl-server.',
  icons: {
    icon: [{ url: '/featherlane-ai-logo.svg', type: 'image/svg+xml' }],
    shortcut: ['/featherlane-ai-logo.svg'],
    apple: [{ url: '/featherlane-ai-logo.svg', type: 'image/svg+xml' }],
  },
};

interface RootLayoutProps {
  children: ReactNode;
}

export default function RootLayout({ children }: RootLayoutProps) {
  return (
    <html
      lang="en"
      className={`${inter.variable} ${ibmPlexMono.variable} ${departureMono.variable} dark`}
      suppressHydrationWarning
    >
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeInitScript }} />
      </head>
      <body className="bg-background text-foreground antialiased">
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem disableTransitionOnChange>
          {children}
          <Toaster />
          <VersionWatcher />
        </ThemeProvider>
      </body>
    </html>
  );
}
