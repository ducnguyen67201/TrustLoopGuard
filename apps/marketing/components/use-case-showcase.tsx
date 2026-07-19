'use client';

import Link from 'next/link';
import { useRef, useState } from 'react';
import type { UseCaseData, UseCaseDemo } from '@/app/use-cases/content';
import type { MarketingLocale } from '@/lib/marketing-locale';
import { UseCaseFlowDemo } from './use-case-flow-demo';

type FeaturedUseCase = UseCaseData & { demo: UseCaseDemo };

const TAB_LABELS: Record<UseCaseDemo['kind'], string> = {
  shell: 'Shell commands',
  email: 'Outbound email',
  spend: 'Agent spend',
};

function hasDemo(useCase: UseCaseData): useCase is FeaturedUseCase {
  return useCase.demo !== undefined;
}

export function UseCaseShowcase({
  useCases,
  locale = 'en',
}: {
  useCases: readonly UseCaseData[];
  locale?: MarketingLocale;
}) {
  const featured = useCases.filter(hasDemo);
  const [activeSlug, setActiveSlug] = useState(featured[0]?.slug);
  const tabRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const active = featured.find((useCase) => useCase.slug === activeSlug) ?? featured[0];

  if (!active) return null;

  const tabLabels =
    locale === 'vi'
      ? { shell: 'Lệnh shell', email: 'Email gửi đi', spend: 'Chi tiêu tác nhân' }
      : TAB_LABELS;
  const showcaseLabel =
    locale === 'vi'
      ? 'Các tình huống sử dụng nổi bật của TrustLoopGuard'
      : 'Featured TrustLoopGuard use cases';
  const exploreLabel = locale === 'vi' ? 'Xem đầy đủ tình huống này' : 'Explore the full use case';

  function selectTab(index: number) {
    const next = featured[index];
    if (!next) return;
    setActiveSlug(next.slug);
    tabRefs.current[index]?.focus();
  }

  return (
    <div className="use-case-showcase">
      <div className="use-case-showcase-tabs" role="tablist" aria-label={showcaseLabel}>
        {featured.map((useCase, index) => {
          const selected = useCase.slug === active.slug;

          return (
            <button
              key={useCase.slug}
              ref={(element) => {
                tabRefs.current[index] = element;
              }}
              id={`${useCase.slug}-tab`}
              type="button"
              role="tab"
              aria-selected={selected}
              aria-controls={`${useCase.slug}-panel`}
              tabIndex={selected ? 0 : -1}
              data-state={selected ? 'active' : 'inactive'}
              onClick={() => setActiveSlug(useCase.slug)}
              onKeyDown={(event) => {
                if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
                  event.preventDefault();
                  selectTab((index + 1) % featured.length);
                }
                if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
                  event.preventDefault();
                  selectTab((index - 1 + featured.length) % featured.length);
                }
                if (event.key === 'Home') {
                  event.preventDefault();
                  selectTab(0);
                }
                if (event.key === 'End') {
                  event.preventDefault();
                  selectTab(featured.length - 1);
                }
              }}
            >
              <span>{useCase.number}</span>
              <strong>{tabLabels[useCase.demo.kind]}</strong>
              <small>{useCase.result}</small>
            </button>
          );
        })}
      </div>

      <section
        id={`${active.slug}-panel`}
        role="tabpanel"
        aria-labelledby={`${active.slug}-tab`}
        className="use-case-showcase-panel"
      >
        <header className="use-case-showcase-heading">
          <div>
            <p>{active.eyebrow}</p>
            <h3>{active.title}</h3>
          </div>
          <div>
            <p>{active.summary}</p>
            <Link href={active.href}>
              {exploreLabel} <span aria-hidden="true">→</span>
            </Link>
          </div>
        </header>
        <UseCaseFlowDemo demo={active.demo} locale={locale} />
      </section>
    </div>
  );
}
