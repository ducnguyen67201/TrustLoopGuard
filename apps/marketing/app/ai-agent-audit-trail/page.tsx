import type { Metadata } from 'next';
import { SeoLandingPage } from '@/components/seo-landing-page';
import { buildLandingMetadata, getLandingPage } from '@/lib/seo';

const page = getLandingPage('ai-agent-audit-trail');

export const metadata: Metadata = buildLandingMetadata(page);

export default function Page() {
  return <SeoLandingPage page={page} />;
}
