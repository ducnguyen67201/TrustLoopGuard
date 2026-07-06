import type { Metadata } from 'next';
import { SeoLandingPage } from '@/components/seo-landing-page';
import { buildLandingMetadata, getLandingPage } from '@/lib/seo';

const page = getLandingPage('ai-agent-spend-controls');

export const metadata: Metadata = buildLandingMetadata(page);

export default function Page() {
  return <SeoLandingPage page={page} />;
}
