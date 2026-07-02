'use client';

import { useEffect, useRef, useState } from 'react';
import { Eyebrow } from './how';

const CAP = 5000;
const PEAK = 6400;
const CAP_MARK_PCT = (CAP / PEAK) * 100;

type Phase = 'screening' | 'approach' | 'escalate' | 'block';

function phaseFor(spend: number): Phase {
  if (spend >= 6000) return 'block';
  if (spend >= CAP) return 'escalate';
  if (spend >= 4000) return 'approach';
  return 'screening';
}

const STATUS: Record<Phase, string> = {
  screening: 'screening · within cap',
  approach: 'approaching cap',
  escalate: 'escalate · approval required',
  block: 'block · declined',
};

export function SpendToVerdict() {
  const trackRef = useRef<HTMLDivElement>(null);
  const [progress, setProgress] = useState(0);
  const [reduced, setReduced] = useState(false);

  useEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      setReduced(true);
      setProgress(1);
      return;
    }

    let raf = 0;
    const update = () => {
      const el = trackRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const total = rect.height - window.innerHeight;
      const p = total > 0 ? Math.min(1, Math.max(0, -rect.top / total)) : 0;
      setProgress(p);
    };
    const onScroll = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(update);
    };
    update();
    window.addEventListener('scroll', onScroll, { passive: true });
    window.addEventListener('resize', onScroll);
    return () => {
      window.removeEventListener('scroll', onScroll);
      window.removeEventListener('resize', onScroll);
      cancelAnimationFrame(raf);
    };
  }, []);

  // Reach the peak (and the BLOCK verdict) by 70% of the scroll, then dwell on it
  // so the final state stays visible regardless of viewport height / sticky release.
  const spend = Math.min(PEAK, Math.round((progress / 0.7) * PEAK));
  const phase = phaseFor(spend);
  const meterPct = Math.min(100, (spend / PEAK) * 100);
  // Card spins through one clean turn over the scroll, landing face-forward at the end.
  const spin = progress * 360;

  return (
    <section id="live" aria-labelledby="live-heading" className="scrolly">
      <div ref={trackRef} className="scrolly-track" data-reduced={reduced}>
        <div className="scrolly-stage">
          <Eyebrow>Live · spend &rarr; verdict</Eyebrow>
          <h2 id="live-heading" className="section-title max-w-3xl">
            Watch the cap fire.
          </h2>
          <p className="section-copy mt-4 max-w-xl">
            As the agent keeps spending, TrustLoopGuard checks every charge against the cap and
            stops it before the money moves. Scroll.
          </p>

          <div className={`sv-card sv-${phase}`} aria-hidden="true">
            <div className="sv-grid">
              <div className="sv-main">
                <div className="sv-head">
                  <span>agent &middot; payments-ap</span>
                  <span className="sv-status">{STATUS[phase]}</span>
                </div>

                <div className="sv-amount tabular-nums">${spend.toLocaleString('en-US')}</div>
                <p className="sv-sub">
                  spent today &middot; cap ${CAP.toLocaleString('en-US')} / day
                </p>

                <div className="sv-meter">
                  <div className="sv-meter-fill" style={{ width: `${meterPct}%` }} />
                  <span className="sv-cap-mark" style={{ left: `${CAP_MARK_PCT}%` }} />
                </div>
                <div className="sv-meter-labels">
                  <span>$0</span>
                  <span className="sv-cap-label" style={{ left: `${CAP_MARK_PCT}%` }}>
                    cap ${(CAP / 1000).toFixed(0)}k
                  </span>
                  <span>${(PEAK / 1000).toFixed(1)}k</span>
                </div>
              </div>

              <div className="sv-cc-stage">
                <div className="sv-cc" style={{ transform: `rotateX(6deg) rotateY(${spin}deg)` }}>
                  <div className="sv-cc-face sv-cc-front">
                    <div className="cc-top">
                      <span className="cc-chip" />
                      <span className="cc-net">VISA</span>
                    </div>
                    <div className="cc-number">···· ···· ···· 4242</div>
                    <div className="cc-bot">
                      <div className="cc-field">
                        <span className="cc-cap">card holder</span>ACME PAYMENTS
                      </div>
                      <div className="cc-field">
                        <span className="cc-cap">exp</span>10 / 27
                      </div>
                    </div>
                    <span className="sv-cc-declined">declined</span>
                  </div>
                  <div className="sv-cc-face sv-cc-back">
                    <div className="sv-cc-mag" />
                    <div className="sv-cc-sig">
                      <span className="sv-cc-strip" />
                      <span className="cc-cap">cvc ···</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="sv-foot">
              {phase === 'screening' && (
                <div className="sv-line sv-line-ok" key="screening">
                  <span className="sv-dot" />
                  every charge checked against the cap before it settles
                </div>
              )}
              {phase === 'approach' && (
                <div className="sv-line sv-line-warn" key="approach">
                  <span className="sv-dot" />
                  approaching the daily cap, watching the next charge
                </div>
              )}
              {phase === 'escalate' && (
                <div className="sv-line sv-verdict-escalate" key="escalate">
                  <span className="sv-chip">escalate</span>
                  over cap, routed to a human for approval
                </div>
              )}
              {phase === 'block' && (
                <div className="sv-line sv-verdict-block" key="block">
                  <span className="sv-stamp">
                    <svg
                      width="11"
                      height="11"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2.5"
                    >
                      <circle cx="12" cy="12" r="9" />
                      <path d="M6 6l12 12" strokeLinecap="round" />
                    </svg>
                    declined
                  </span>
                  <div>
                    <strong>block</strong> &middot; exceeds ${CAP.toLocaleString('en-US')}/day cap
                    by ${(spend - CAP).toLocaleString('en-US')}
                    <div className="sv-trace">trace tr_9f3a71c4e2 &middot; &#10003; ed25519</div>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
