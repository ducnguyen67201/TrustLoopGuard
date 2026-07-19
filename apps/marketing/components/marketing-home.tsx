import type { MarketingLocale } from '@/lib/marketing-locale';
import { ControlLoop } from './control-loop';
import { Cta } from './cta';
import { Evidence } from './evidence';
import { Footer } from './footer';
import { Hero } from './hero';
import { Nav } from './nav';
import { Sdk } from './sdk';
import { TrustStory } from './trust-story';
import { Why } from './why';

export function MarketingHome({ locale }: { locale: MarketingLocale }) {
  return (
    <div lang={locale}>
      <Nav locale={locale} />
      <main>
        <Hero locale={locale} />
        <ControlLoop locale={locale} />
        <Evidence locale={locale} />
        <Sdk locale={locale} />
        <TrustStory locale={locale} />
        <Why locale={locale} />
        <Cta locale={locale} />
      </main>
      <Footer locale={locale} />
    </div>
  );
}
