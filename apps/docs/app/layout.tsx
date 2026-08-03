import './global.css';
import { RootProvider } from 'fumadocs-ui/provider/next';
import { IBM_Plex_Mono, Instrument_Sans } from 'next/font/google';
import type { ReactNode } from 'react';

const instrumentSans = Instrument_Sans({
  subsets: ['latin'],
  variable: '--font-instrument-sans',
  display: 'swap',
});

const ibmPlexMono = IBM_Plex_Mono({
  subsets: ['latin'],
  variable: '--font-ibm-plex-mono',
  display: 'swap',
  weight: ['400', '500', '600', '700'],
});

export const metadata = {
  title: 'Featherlane AI docs',
  description:
    'Featherlane AI is an open-source policy enforcement runtime. These docs cover concepts, the CLI, the HTTP API, and the SDKs.',
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html
      lang="en"
      className={`${instrumentSans.variable} ${ibmPlexMono.variable}`}
      suppressHydrationWarning
    >
      <body className="flex min-h-screen flex-col font-sans">
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}
