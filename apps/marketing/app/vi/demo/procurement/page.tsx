import type { Metadata } from 'next';

import { ProcurementDemoPageContent } from '@/app/demo/procurement/procurement-page';
import { SITE_NAME, SITE_URL } from '@/lib/seo';

const TITLE = 'Demo tác nhân mua sắm AI an toàn';
const DESCRIPTION =
  'Trò chuyện với tác nhân mua sắm OpenAI trực tiếp và xem Featherlane AI cho phép, giữ để phê duyệt hoặc chặn đơn mua hàng trước khi thực thi.';

export const metadata: Metadata = {
  title: { absolute: `${TITLE} | ${SITE_NAME}` },
  description: DESCRIPTION,
  alternates: {
    canonical: '/vi/demo/procurement',
    languages: {
      en: '/demo/procurement',
      vi: '/vi/demo/procurement',
    },
  },
  openGraph: {
    title: TITLE,
    description: DESCRIPTION,
    url: `${SITE_URL}/vi/demo/procurement`,
    siteName: SITE_NAME,
    locale: 'vi_VN',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: TITLE,
    description: DESCRIPTION,
  },
};

export default function VietnameseProcurementDemoPage() {
  return <ProcurementDemoPageContent locale="vi" />;
}
