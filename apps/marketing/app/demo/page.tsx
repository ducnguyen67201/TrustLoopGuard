import type { Metadata } from 'next';
import { REFUND_PAGE_COPY } from './refund-content';
import { RefundDemoPageContent } from './refund-page';

const copy = REFUND_PAGE_COPY.en;

export const metadata: Metadata = {
  title: copy.title,
  description: copy.description,
  alternates: {
    canonical: '/demo',
    languages: { en: '/demo', vi: '/vi/demo' },
  },
};

export default function DemoPage() {
  return <RefundDemoPageContent locale="en" />;
}
