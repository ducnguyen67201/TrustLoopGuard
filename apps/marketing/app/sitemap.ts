import type { MetadataRoute } from 'next';
import { absoluteUrl, landingPages } from '@/lib/seo';

const UPDATED_AT = new Date('2026-07-06T00:00:00.000Z');

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    {
      url: absoluteUrl('/'),
      lastModified: UPDATED_AT,
      changeFrequency: 'weekly',
      priority: 1,
    },
    ...landingPages.map((page) => ({
      url: absoluteUrl(`/${page.slug}`),
      lastModified: UPDATED_AT,
      changeFrequency: 'monthly' as const,
      priority: 0.8,
    })),
  ];
}
