import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { Inter, Space_Grotesk } from 'next/font/google';
import localFont from 'next/font/local';
import { GeistMono } from 'geist/font/mono';
import { env } from '@/env';
import {
  DEFAULT_DESCRIPTION,
  SITE_NAME,
  SITE_URL,
  organizationJsonLd,
  serializeJsonLd,
  softwareApplicationJsonLd,
  websiteJsonLd,
} from '@/lib/seo';
import './globals.css';

const inter = Inter({ subsets: ['latin'], variable: '--font-inter', display: 'swap' });
const spaceGrotesk = Space_Grotesk({
  subsets: ['latin'],
  variable: '--font-display',
  display: 'swap',
});
// Departure Mono (SIL OFL) — pixel-grid mono for terminal/decision surfaces.
const departureMono = localFont({
  src: '../public/fonts/DepartureMono-Regular.woff2',
  variable: '--font-pixel',
  display: 'swap',
  weight: '400',
});

export const metadata: Metadata = {
  title: {
    default: 'TrustLoopGuard — Approval infrastructure for AI agents',
    template: `%s | ${SITE_NAME}`,
  },
  description: DEFAULT_DESCRIPTION,
  metadataBase: new URL(SITE_URL),
  alternates: {
    canonical: '/',
    languages: {
      en: '/',
      vi: '/vi',
    },
  },
  openGraph: {
    title: 'TrustLoopGuard — Approval infrastructure for AI agents',
    description: DEFAULT_DESCRIPTION,
    url: SITE_URL,
    siteName: SITE_NAME,
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'TrustLoopGuard — Approval infrastructure for AI agents',
    description: DEFAULT_DESCRIPTION,
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
  const gtmId = env.NEXT_PUBLIC_GTM_ID;

  return (
    <html
      lang="en"
      className={`${inter.variable} ${spaceGrotesk.variable} ${GeistMono.variable} ${departureMono.variable}`}
    >
      {gtmId ? (
        <script
          dangerouslySetInnerHTML={{
            __html: `
(function(w,d,s,l,i){w[l]=w[l]||[];w[l].push({'gtm.start':
new Date().getTime(),event:'gtm.js'});var f=d.getElementsByTagName(s)[0],
j=d.createElement(s),dl=l!='dataLayer'?'&l='+l:'';j.async=true;j.src=
'https://www.googletagmanager.com/gtm.js?id='+i+dl;f.parentNode.insertBefore(j,f);
})(window,document,'script','dataLayer','${gtmId}');`,
          }}
        />
      ) : null}
      <body id="top" className="min-h-svh font-sans">
        {gtmId ? (
          <noscript>
            <iframe
              src={`https://www.googletagmanager.com/ns.html?id=${gtmId}`}
              height="0"
              width="0"
              style={{ display: 'none', visibility: 'hidden' }}
            />
          </noscript>
        ) : null}
        {children}
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: serializeJsonLd(organizationJsonLd()) }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: serializeJsonLd(softwareApplicationJsonLd()) }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: serializeJsonLd(websiteJsonLd()) }}
        />
      </body>
    </html>
  );
}
