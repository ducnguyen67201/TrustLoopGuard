import type { Metadata } from 'next';
import Link from 'next/link';

import sharedStyles from '@/app/demo/demo.module.css';
import { ProcurementDemo } from '@/app/demo/procurement/procurement-demo';
import styles from '@/app/demo/procurement/procurement.module.css';
import { SITE_NAME, SITE_URL } from '@/lib/seo';

const TITLE = 'Demo tác nhân mua sắm AI an toàn';
const DESCRIPTION =
  'Trò chuyện với tác nhân mua sắm OpenAI trực tiếp và xem TrustLoopGuard cho phép, giữ để phê duyệt hoặc chặn đơn mua hàng trước khi thực thi.';

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
  return (
    <main lang="vi" className={sharedStyles['page']}>
      <header className={sharedStyles['topbar']}>
        <Link href="/vi" className={sharedStyles['wordmark']} aria-label="Trang chủ TrustLoopGuard">
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" />
          <span>TrustLoopGuard</span>
        </Link>
        <div className={sharedStyles['stackStatus']}>
          <span>OpenAI Agents SDK</span>
          <i aria-hidden="true" />
          <span>Rust API</span>
          <i aria-hidden="true" />
          <span>Mua sắm demo</span>
        </div>
        <a href="https://github.com/ducnguyen67201/TrustLoopGuard" target="_blank" rel="noreferrer">
          Xem mã nguồn <span aria-hidden="true">↗</span>
        </a>
      </header>

      <section
        className={`${sharedStyles['intro']} ${styles['procurementIntro']}`}
        aria-labelledby="procurement-demo-title"
      >
        <div>
          <p className={sharedStyles['eyebrow']}>Demo trực tiếp tác nhân mua sắm</p>
          <h1 id="procurement-demo-title" className={styles['title']}>
            OpenAI đề xuất. TrustLoopGuard quyết định trước khi hệ thống mua sắm thực thi.
          </h1>
        </div>
        <div className={sharedStyles['introCopy']}>
          <p>
            Yêu cầu tác nhân trực tiếp tìm nguồn hàng. Tác nhân có thể tìm trong danh mục demo,
            nhưng mọi đơn mua hàng phải vượt qua đánh giá chính sách TrustLoopGuard thực trước khi
            hệ thống mua sắm được gọi.
          </p>
          <small>Được xây dựng với OpenAI Agents SDK và bảo vệ bởi TrustLoopGuard.</small>
        </div>
      </section>

      <div className={styles['dataNotice']} role="note">
        Chỉ sử dụng danh mục demo. Không nhập thông tin mua sắm, nhà cung cấp hoặc thương mại mật.
      </div>

      <ProcurementDemo locale="vi" />

      <footer className={sharedStyles['demoFooter']}>
        <p>Đặt ranh giới kiểm soát xác định cho công cụ mua sắm của tác nhân AI.</p>
        <Link href="/vi">
          Khám phá TrustLoopGuard <span aria-hidden="true">→</span>
        </Link>
      </footer>
    </main>
  );
}
