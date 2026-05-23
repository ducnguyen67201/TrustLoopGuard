import { Nav } from '@/components/nav';
import { Hero } from '@/components/hero';
import { How } from '@/components/how';
import { Sdk } from '@/components/sdk';
import { Why } from '@/components/why';
import { Cta } from '@/components/cta';
import { Footer } from '@/components/footer';
import { Monitoring } from '@/components/monitoring';
import { Verdicts } from '@/components/verdicts';

export default function Page() {
  return (
    <>
      <Nav />
      <main>
        <Hero />
        <How />
        <Verdicts />
        <Sdk />
        <Monitoring />
        <Why />
        <Cta />
      </main>
      <Footer />
    </>
  );
}
