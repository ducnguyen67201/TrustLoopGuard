import './global.css';
import { RootProvider } from 'fumadocs-ui/provider/next';
import localFont from 'next/font/local';
import type { ReactNode } from 'react';

// Departure Mono (SIL OFL) — pixel-grid mono for code blocks, matching the app.
const departureMono = localFont({
  src: '../public/fonts/DepartureMono-Regular.woff2',
  variable: '--font-pixel',
  display: 'swap',
  weight: '400',
});

export const metadata = {
  title: 'Featherlane AI docs',
  description:
    'Featherlane AI is an open-source policy enforcement runtime. These docs cover concepts, the CLI, the HTTP API, and the SDKs.',
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={departureMono.variable} suppressHydrationWarning>
      <body className="flex min-h-screen flex-col">
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}
