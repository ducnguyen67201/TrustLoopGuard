import type { MetadataRoute } from 'next';
import { USE_CASES } from '@/app/use-cases/content';
import { absoluteUrl, landingPages } from '@/lib/seo';

const HOME_LAST_MODIFIED = new Date('2026-07-19');
const USE_CASES_LAST_MODIFIED = new Date('2026-07-16');
const REFUND_DEMO_LAST_MODIFIED = new Date('2026-07-20');
const HEALTHCARE_DEMO_LAST_MODIFIED = new Date('2026-07-19');
const PROCUREMENT_DEMO_LAST_MODIFIED = new Date('2026-07-19');

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    {
      url: absoluteUrl('/'),
      lastModified: HOME_LAST_MODIFIED,
      changeFrequency: 'weekly',
      priority: 1,
      alternates: {
        languages: {
          en: absoluteUrl('/'),
          vi: absoluteUrl('/vi'),
        },
      },
    },
    {
      url: absoluteUrl('/vi'),
      lastModified: HOME_LAST_MODIFIED,
      changeFrequency: 'weekly',
      priority: 0.95,
      alternates: {
        languages: {
          en: absoluteUrl('/'),
          vi: absoluteUrl('/vi'),
        },
      },
    },
    {
      url: absoluteUrl('/use-cases'),
      lastModified: USE_CASES_LAST_MODIFIED,
      changeFrequency: 'monthly',
      priority: 0.9,
    },
    {
      url: absoluteUrl('/demo'),
      lastModified: REFUND_DEMO_LAST_MODIFIED,
      changeFrequency: 'monthly',
      priority: 0.85,
      alternates: {
        languages: {
          en: absoluteUrl('/demo'),
          vi: absoluteUrl('/vi/demo'),
        },
      },
    },
    {
      url: absoluteUrl('/vi/demo'),
      lastModified: REFUND_DEMO_LAST_MODIFIED,
      changeFrequency: 'monthly',
      priority: 0.85,
      alternates: {
        languages: {
          en: absoluteUrl('/demo'),
          vi: absoluteUrl('/vi/demo'),
        },
      },
    },
    {
      url: absoluteUrl('/demo/healthcare'),
      lastModified: HEALTHCARE_DEMO_LAST_MODIFIED,
      changeFrequency: 'monthly',
      priority: 0.85,
      alternates: {
        languages: {
          en: absoluteUrl('/demo/healthcare'),
          vi: absoluteUrl('/vi/demo/healthcare'),
        },
      },
    },
    {
      url: absoluteUrl('/vi/demo/healthcare'),
      lastModified: HEALTHCARE_DEMO_LAST_MODIFIED,
      changeFrequency: 'monthly',
      priority: 0.85,
      alternates: {
        languages: {
          en: absoluteUrl('/demo/healthcare'),
          vi: absoluteUrl('/vi/demo/healthcare'),
        },
      },
    },
    {
      url: absoluteUrl('/demo/procurement'),
      lastModified: PROCUREMENT_DEMO_LAST_MODIFIED,
      changeFrequency: 'monthly',
      priority: 0.85,
      alternates: {
        languages: {
          en: absoluteUrl('/demo/procurement'),
          vi: absoluteUrl('/vi/demo/procurement'),
        },
      },
    },
    {
      url: absoluteUrl('/vi/demo/procurement'),
      lastModified: PROCUREMENT_DEMO_LAST_MODIFIED,
      changeFrequency: 'monthly',
      priority: 0.85,
      alternates: {
        languages: {
          en: absoluteUrl('/demo/procurement'),
          vi: absoluteUrl('/vi/demo/procurement'),
        },
      },
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
