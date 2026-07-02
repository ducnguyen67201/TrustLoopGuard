'use client';

import { useRef } from 'react';

const TILT_X = 18;
const TILT_Y = 22;

/** Hero credit card that tilts toward the cursor on hover. */
export function HeroCard() {
  const ref = useRef<HTMLDivElement>(null);

  const handleMove = (e: React.MouseEvent) => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const x = (e.clientX - r.left) / r.width - 0.5;
    const y = (e.clientY - r.top) / r.height - 0.5;
    el.style.transform = `rotateX(${(-y * TILT_X).toFixed(1)}deg) rotateY(${(x * TILT_Y).toFixed(1)}deg)`;
  };

  const handleLeave = () => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    if (ref.current) ref.current.style.transform = '';
  };

  return (
    <div className="cc-hero-stage" onMouseMove={handleMove} onMouseLeave={handleLeave}>
      <div ref={ref} className="cc cc-hero" aria-hidden="true">
        <div className="cc-shine" />
        <div className="cc-top">
          <span className="cc-chip" />
          <span className="cc-net">TLG&nbsp;PAY</span>
        </div>
        <div className="cc-number">···· ···· ···· 4242</div>
        <div className="cc-bot">
          <span className="cc-field">
            <span className="cc-cap">card holder</span>
            ACME PAYMENTS
          </span>
          <span className="cc-field">
            <span className="cc-cap">exp</span>
            10 / 27
          </span>
        </div>
      </div>
    </div>
  );
}
