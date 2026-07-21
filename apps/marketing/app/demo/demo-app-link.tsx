'use client';

import { usePathname } from 'next/navigation';

import { MarketingEventLink } from '@/components/marketing-event-link';
import { APP_URL } from '@/lib/app-url';
import type { MarketingLocale } from '@/lib/marketing-locale';

import styles from './demo.module.css';

const COPY = {
  en: 'Go to the app',
  vi: 'Vào ứng dụng',
} as const;

export function DemoAppLink({ locale }: { locale: MarketingLocale }) {
  const page = usePathname() || (locale === 'vi' ? '/vi/demo' : '/demo');
  const label = COPY[locale];

  return (
    <MarketingEventLink
      href={APP_URL}
      className={styles['demoAppLink']}
      event="app_click"
      eventParams={{ page, location: 'demo_nav', label }}
    >
      {label} <span aria-hidden="true">→</span>
    </MarketingEventLink>
  );
}
