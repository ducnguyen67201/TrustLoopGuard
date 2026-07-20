import type { Metadata } from 'next';

import { HEALTHCARE_PAGE_COPY } from '@/app/demo/healthcare/content';
import { HealthcareDemoPageContent } from '@/app/demo/healthcare/healthcare-page';

const copy = HEALTHCARE_PAGE_COPY.vi;

export const metadata: Metadata = {
  title: copy.title,
  description: copy.description,
  alternates: {
    canonical: '/vi/demo/healthcare',
    languages: { en: '/demo/healthcare', vi: '/vi/demo/healthcare' },
  },
  openGraph: {
    title: copy.title,
    description: copy.description,
    url: '/vi/demo/healthcare',
    locale: 'vi_VN',
    type: 'website',
  },
};

export default function VietnameseHealthcareDemoPage() {
  return <HealthcareDemoPageContent locale="vi" />;
}
