'use client';

import Link from 'next/link';
import { useCallback, useEffect, useRef, useState } from 'react';
import { USE_CASE_MENU_CLOSE_DELAY_MS, USE_CASE_NAV_GROUPS } from '@/app/use-cases/content';

export function UseCaseNav() {
  const [isOpen, setIsOpen] = useState(false);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const triggerRef = useRef<HTMLAnchorElement>(null);

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
        Use cases <span className="site-nav-dropdown-chevron" aria-hidden="true" />
      </Link>
      <div id="use-cases-menu" className="site-nav-dropdown-menu" aria-label="Use cases">
        <div className="site-nav-mega-header">
          <div>
            <small>Use cases</small>
            <strong>Choose where TrustLoopGuard controls the action.</strong>
          </div>
          <Link href={USE_CASE_NAV_GROUPS.overview.href}>
            View all use cases <span aria-hidden="true">→</span>
          </Link>
        </div>
        <ul className="site-nav-mega-grid">
          {USE_CASE_NAV_GROUPS.details.map((item, index) => (
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
