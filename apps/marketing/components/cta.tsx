import { BOOK_MEETING_URL, DOCS_URL, GITHUB_URL } from '@/lib/github';

export function Cta() {
  return (
    <section aria-labelledby="cta-heading" className="section">
      <div className="border border-[var(--color-line-strong)] bg-[var(--color-ink)] p-8 text-white md:p-10">
        <p className="eyebrow text-white/60">Ship the check</p>
        <h2
          id="cta-heading"
          className="mt-5 max-w-3xl text-4xl font-semibold leading-tight md:text-5xl"
        >
          Add a policy check before your agent acts.
        </h2>
        <p className="mt-5 max-w-2xl text-base leading-7 text-white/70">
          Start with the SDK, run the Rust service, and inspect decisions in the dashboard when your
          agent hits policy boundaries.
        </p>
        <div className="mt-8 flex flex-col gap-3 sm:flex-row">
          <a
            href={BOOK_MEETING_URL}
            target="_blank"
            rel="noreferrer"
            className="button-accent h-11 px-5"
          >
            Book a meeting
          </a>
          <a href={GITHUB_URL} className="button-invert h-11 px-5">
            Clone on GitHub
          </a>
          <a href={DOCS_URL} className="button-dark h-11 px-5">
            Read the docs
          </a>
        </div>
      </div>
    </section>
  );
}
