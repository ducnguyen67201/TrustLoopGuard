import type { Metadata } from 'next';
import { SeoLandingPage } from '@/components/seo-landing-page';
import { buildLandingMetadata, getLandingPage } from '@/lib/seo';

const page = getLandingPage('ai-agent-payment-gateway');

export const metadata: Metadata = buildLandingMetadata(page);

export default function Page() {
  return <SeoLandingPage page={page} />;
}
