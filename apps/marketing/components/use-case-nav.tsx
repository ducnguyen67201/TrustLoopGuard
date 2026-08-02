'use client';

import Link from 'next/link';
import { useCallback, useEffect, useRef, useState } from 'react';
import { USE_CASE_MENU_CLOSE_DELAY_MS, USE_CASE_NAV_GROUPS } from '@/app/use-cases/content';
import type { MarketingLocale } from '@/lib/marketing-locale';

const VI_DETAILS = [
  { label: 'An toàn lệnh shell', detail: 'Từ chối hoặc phê duyệt trước khi thực thi' },
  { label: 'Email gửi đi', detail: 'Cho phép hoặc viết lại trước khi gửi' },
  { label: 'Hạn mức chi tiêu của tác nhân', detail: 'Cho phép, giữ hoặc từ chối thanh toán' },
  { label: 'Chi phí suy luận AI', detail: 'Đo lường, cảnh báo và đặt trần cứng' },
  { label: 'Thanh toán x402 của tác nhân', detail: 'Cấp quyền trước khi ví ký' },
  { label: 'Cấp quyền hành động', detail: 'Kiểm soát trước điểm không thể quay lại' },
] as const;

export function UseCaseNav({ locale = 'en' }: { locale?: MarketingLocale }) {
  const [isOpen, setIsOpen] = useState(false);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const triggerRef = useRef<HTMLAnchorElement>(null);
  const copy =
    locale === 'vi'
      ? {
          label: 'Tình huống sử dụng',
          heading: 'Chọn nơi Featherlane AI kiểm soát hành động.',
          viewAll: 'Xem tất cả tình huống',
          details: USE_CASE_NAV_GROUPS.details.map((item, index) => ({
            ...item,
            ...VI_DETAILS[index],
          })),
        }
      : {
          label: 'Use cases',
          heading: 'Choose where Featherlane AI controls the action.',
          viewAll: 'View all use cases',
          details: USE_CASE_NAV_GROUPS.details,
        };

  const cancelClose = useCallback(() => {
    if (closeTimer.current) {
      clearTimeout(closeTimer.current);
      closeTimer.current = null;
    }
  }, []);

  const openMenu = useCallback(() => {
    cancelClose();
    setIsOpen(true);
  }, [cancelClose]);

  const closeMenu = useCallback(() => {
    cancelClose();
    setIsOpen(false);
  }, [cancelClose]);

  const scheduleClose = useCallback(() => {
    cancelClose();
    closeTimer.current = setTimeout(closeMenu, USE_CASE_MENU_CLOSE_DELAY_MS);
  }, [cancelClose, closeMenu]);

  useEffect(() => cancelClose, [cancelClose]);

  return (
    <li
      className="site-nav-dropdown"
      data-open={isOpen ? 'true' : 'false'}
      onPointerEnter={openMenu}
      onPointerLeave={scheduleClose}
      onFocusCapture={openMenu}
      onBlurCapture={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
          scheduleClose();
        }
      }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.preventDefault();
          closeMenu();
          triggerRef.current?.focus();
        }
      }}
    >
      <Link
        ref={triggerRef}
        href="/use-cases"
        className="site-nav-dropdown-trigger"
        aria-haspopup="true"
        aria-expanded={isOpen}
        aria-controls="use-cases-menu"
      >
        {copy.label} <span className="site-nav-dropdown-chevron" aria-hidden="true" />
      </Link>
      <div id="use-cases-menu" className="site-nav-dropdown-menu" aria-label={copy.label}>
        <div className="site-nav-mega-header">
          <div>
            <small>{copy.label}</small>
            <strong>{copy.heading}</strong>
          </div>
          <Link href={USE_CASE_NAV_GROUPS.overview.href}>
            {copy.viewAll} <span aria-hidden="true">→</span>
          </Link>
        </div>
        <ul className="site-nav-mega-grid">
          {copy.details.map((item, index) => (
            <li key={item.href}>
              <Link href={item.href}>
                <small>0{index + 1}</small>
                <strong>{item.label}</strong>
                <span>{item.detail}</span>
                <i aria-hidden="true">→</i>
              </Link>
            </li>
          ))}
        </ul>
      </div>
    </li>
  );
}
