import type { MarketingLocale } from '@/lib/marketing-locale';
import { RefundDemo } from '@/app/demo/refund-demo';

const COPY = {
  en: {
    eyebrow: 'Live demo',
    heading: 'Try it live.',
    intro: 'Request a refund. Watch policy decide.',
  },
  vi: {
    eyebrow: 'Bản demo trực tiếp',
    heading: 'Thử trực tiếp.',
    intro: 'Yêu cầu hoàn tiền. Xem chính sách quyết định.',
  },
} as const;

export function HomeDemo({ locale }: { locale: MarketingLocale }) {
  const copy = COPY[locale];

  return (
    <section id="demo" className="home-demo" aria-labelledby="home-demo-heading">
      <div className="home-demo-heading">
        <div>
          <p className="eyebrow">{copy.eyebrow}</p>
          <h2 id="home-demo-heading">{copy.heading}</h2>
        </div>
        <p>{copy.intro}</p>
      </div>
      <RefundDemo locale={locale} />
    </section>
  );
}
