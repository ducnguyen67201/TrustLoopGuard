import { Nav } from '@/components/nav';
import { Hero } from '@/components/hero';
import { ControlLoop } from '@/components/control-loop';
import { Evidence } from '@/components/evidence';
import { Sdk } from '@/components/sdk';
import { Why } from '@/components/why';
import { TrustStory } from '@/components/trust-story';
import { Cta } from '@/components/cta';
import { Footer } from '@/components/footer';
import { ScrollTopButton } from '@/components/scroll-top-button';

export default function Page() {
  return (
    <>
      <Nav />
      <main>
        <Hero />
        <TrustStory />
        <ControlLoop />
        <Evidence />
        <Sdk />
        <Why />
        <Cta />
      </main>
      <Footer />
      <ScrollTopButton />
    </>
  );
}
