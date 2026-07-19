export type MarketingLocale = 'en' | 'vi';

export function localizedHomeHref(locale: MarketingLocale, hash = ''): string {
  return locale === 'vi' ? `/vi${hash}` : `/${hash}`;
}
