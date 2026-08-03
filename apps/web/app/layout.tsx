import type { Metadata } from 'next';
import type { ReactNode } from 'react';
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

// Departure Mono (SIL OFL) is the only product typeface.
const primaryFont = localFont({
  src: '../public/fonts/DepartureMono-Regular.woff2',
  variable: '--font-primary',
  display: 'swap',
  weight: '400',
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
    <html lang="en" className={`${primaryFont.variable} dark`} suppressHydrationWarning>
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
