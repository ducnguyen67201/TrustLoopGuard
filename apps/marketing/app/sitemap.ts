import type { MetadataRoute } from 'next';
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
    ...landingPages.map((page) => ({
      url: absoluteUrl(`/${page.slug}`),
      lastModified: new Date(page.lastModified),
      changeFrequency: 'monthly' as const,
      priority: 0.8,
    })),
  ];
}
