import type { Metadata } from 'next';

import { REFUND_PAGE_COPY } from '@/app/demo/refund-content';
import { RefundDemoPageContent } from '@/app/demo/refund-page';
import { SITE_NAME, SITE_URL } from '@/lib/seo';

const copy = REFUND_PAGE_COPY.vi;

export const metadata: Metadata = {
  title: { absolute: `${copy.title} | ${SITE_NAME}` },
  description: copy.description,
  alternates: {
    canonical: '/vi/demo',
    languages: { en: '/demo', vi: '/vi/demo' },
  },
  openGraph: {
    title: copy.title,
    description: copy.description,
    url: `${SITE_URL}/vi/demo`,
    siteName: SITE_NAME,
    locale: 'vi_VN',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: copy.title,
    description: copy.description,
  },
};

export default function VietnameseRefundDemoPage() {
  return <RefundDemoPageContent locale="vi" />;
}
