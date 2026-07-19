import type { Metadata } from 'next';

import { HEALTHCARE_PAGE_COPY } from './content';
import { HealthcareDemoPageContent } from './healthcare-page';

const copy = HEALTHCARE_PAGE_COPY.en;

export const metadata: Metadata = {
  title: copy.title,
  description: copy.description,
  alternates: {
    canonical: '/demo/healthcare',
    languages: { en: '/demo/healthcare', vi: '/vi/demo/healthcare' },
  },
};

export default function HealthcareDemoPage() {
  return <HealthcareDemoPageContent locale="en" />;
}
