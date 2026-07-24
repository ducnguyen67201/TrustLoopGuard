import type { Metadata } from 'next';
import { MarketingHome } from '@/components/marketing-home';
import { SITE_NAME, SITE_URL } from '@/lib/seo';

const TITLE = 'TrustLoopGuard — Thẩm định rủi ro cho hành động của tác nhân AI';
const DESCRIPTION =
  'Định giá hành động quan trọng của tác nhân AI trước khi thực thi, cấp quyền theo điều khoản đã thỏa thuận và gắn mọi kết quả với hồ sơ rủi ro có thể kiểm tra.';

export const metadata: Metadata = {
  title: { absolute: TITLE },
  description: DESCRIPTION,
  alternates: {
    canonical: '/vi',
    languages: {
      en: '/',
      vi: '/vi',
    },
  },
  openGraph: {
    title: TITLE,
    description: DESCRIPTION,
    url: `${SITE_URL}/vi`,
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

export default function VietnamesePage() {
  return <MarketingHome locale="vi" />;
}
