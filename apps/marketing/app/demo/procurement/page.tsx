import type { Metadata } from 'next';

import { ProcurementDemoPageContent } from './procurement-page';

export const metadata: Metadata = {
  title: 'Secure AI Procurement Agent Demo',
  description:
    'Chat with a live OpenAI procurement agent and watch Featherlane AI permit, hold, or block purchase orders before they execute.',
  alternates: {
    canonical: '/demo/procurement',
    languages: {
      en: '/demo/procurement',
      vi: '/vi/demo/procurement',
    },
  },
};

export default function ProcurementDemoPage() {
  return <ProcurementDemoPageContent locale="en" />;
}
