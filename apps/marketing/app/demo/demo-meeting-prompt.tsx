'use client';

import { useCallback, useEffect, useReducer, useRef } from 'react';

import { MarketingEventLink } from '@/components/marketing-event-link';
import { BOOK_MEETING_URL } from '@/lib/github';
import type { MarketingLocale } from '@/lib/marketing-locale';

import {
  initialDemoMeetingPromptState,
  reduceDemoMeetingPromptState,
} from './demo-meeting-prompt-state';
import styles from './demo.module.css';

const COPY = {
  en: {
    kicker: 'One more thing',
    title: 'Hey, you seem to like the product.',
    description:
      'Want to talk through how Featherlane AI could work for your team? Pick a time on Duc’s calendar.',
    book: 'Book a call with Duc',
    continue: 'Keep exploring',
    close: 'Close meeting prompt',
  },
  vi: {
    kicker: 'Một điều nữa',
    title: 'Có vẻ bạn đang thích những gì mình thấy.',
    description:
      'Bạn muốn trao đổi thêm về cách Featherlane AI có thể hỗ trợ đội ngũ của mình không? Hãy chọn thời gian phù hợp trên lịch của Duc.',
    book: 'Đặt lịch với Duc',
    continue: 'Tiếp tục khám phá',
    close: 'Đóng lời mời đặt lịch',
  },
} as const;

export function useDemoMeetingPrompt() {
  const [state, dispatch] = useReducer(reduceDemoMeetingPromptState, initialDemoMeetingPromptState);

  const recordCompletedInteraction = useCallback(() => {
    dispatch({ type: 'interaction_completed' });
  }, []);
  const dismissMeetingPrompt = useCallback(() => {
    dispatch({ type: 'dismissed' });
  }, []);

  return {
    isMeetingPromptOpen: state.isOpen,
    recordCompletedInteraction,
    dismissMeetingPrompt,
  };
}

export function DemoMeetingPrompt({
  open,
  onClose,
  page,
  locale = 'en',
}: {
  open: boolean;
  onClose: () => void;
  page: string;
  locale?: MarketingLocale;
}) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);
  const copy = COPY[locale];

  useEffect(() => {
    const dialog = dialogRef.current;
    if (dialog === null) return;

    if (open && !dialog.open) {
      previousFocusRef.current =
        document.activeElement instanceof HTMLElement ? document.activeElement : null;
      dialog.showModal();
      return;
    }

    if (!open && dialog.open) {
      dialog.close();
      previousFocusRef.current?.focus();
    }
  }, [open]);

  return (
    <dialog
      ref={dialogRef}
      className={styles['meetingPrompt']}
      aria-labelledby="demo-meeting-prompt-title"
      aria-describedby="demo-meeting-prompt-description"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className={styles['meetingPromptCard']}>
        <button
          type="button"
          className={styles['meetingPromptClose']}
          onClick={onClose}
          aria-label={copy.close}
        >
          <span aria-hidden="true">×</span>
        </button>
        <p className={styles['meetingPromptKicker']}>{copy.kicker}</p>
        <h2 id="demo-meeting-prompt-title">{copy.title}</h2>
        <p id="demo-meeting-prompt-description">{copy.description}</p>
        <div className={styles['meetingPromptActions']}>
          <MarketingEventLink
            href={BOOK_MEETING_URL}
            target="_blank"
            autoFocus
            className={styles['meetingPromptPrimary']}
            event="book_meeting_click"
            eventParams={{
              page,
              location: 'demo_meeting_prompt',
              label: copy.book,
            }}
          >
            {copy.book} <span aria-hidden="true">↗</span>
          </MarketingEventLink>
          <button type="button" className={styles['meetingPromptSecondary']} onClick={onClose}>
            {copy.continue}
          </button>
        </div>
      </div>
    </dialog>
  );
}
