import type { Metadata } from 'next';
import { MarketingHome } from '@/components/marketing-home';
import { SITE_NAME, SITE_URL } from '@/lib/seo';

const TITLE = 'TrustLoopGuard — Kiểm soát tác nhân AI trong thời gian chạy';
const DESCRIPTION =
  'Kiểm tra đầu ra và hành động do tác nhân AI đề xuất theo chính sách trước khi thực thi. Nhận quyết định cho phép, từ chối, chuyển đổi hoặc yêu cầu phê duyệt, kèm lý do và mã truy vết.';

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
