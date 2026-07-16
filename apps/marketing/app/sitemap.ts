import type { MetadataRoute } from 'next';
import { USE_CASES } from '@/app/use-cases/content';
import { absoluteUrl, landingPages } from '@/lib/seo';

const HOME_LAST_MODIFIED = new Date('2026-07-16');
const USE_CASES_LAST_MODIFIED = new Date('2026-07-16');

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    {
      url: absoluteUrl('/'),
      lastModified: HOME_LAST_MODIFIED,
      changeFrequency: 'weekly',
      priority: 1,
    },
    {
      url: absoluteUrl('/use-cases'),
      lastModified: USE_CASES_LAST_MODIFIED,
      changeFrequency: 'monthly',
      priority: 0.9,
    },
    ...USE_CASES.map((useCase) => ({
      url: absoluteUrl(useCase.href),
      lastModified: USE_CASES_LAST_MODIFIED,
      changeFrequency: 'monthly' as const,
      priority: 0.85,
    })),
    ...landingPages.map((page) => ({
      url: absoluteUrl(`/${page.slug}`),
      lastModified: new Date(page.lastModified),
      changeFrequency: 'monthly' as const,
      priority: 0.8,
    })),
  ];
}
