import type { Metadata } from 'next';
import { SeoLandingPage } from '@/components/seo-landing-page';
import { buildLandingMetadata, getLandingPage } from '@/lib/seo';

const page = getLandingPage('shopping-agent-checkout');

export const metadata: Metadata = buildLandingMetadata(page);

export default function Page() {
  return <SeoLandingPage page={page} />;
}
