import type { MetadataRoute } from 'next';
import { USE_CASES } from '@/app/use-cases/content';
import { absoluteUrl, landingPages } from '@/lib/seo';

const HOME_LAST_MODIFIED = new Date('2026-07-06');

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
      lastModified: new Date('2026-07-13'),
      changeFrequency: 'monthly',
      priority: 0.9,
    },
    ...USE_CASES.map((useCase) => ({
      url: absoluteUrl(useCase.href),
      lastModified: new Date('2026-07-13'),
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
