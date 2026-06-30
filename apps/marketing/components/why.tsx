import { Eyebrow } from './how';

const USE_CASES = [
  {
    title: 'Shopping & travel agents',
    body: 'Cap the cart at $500, block the duplicate $1,200 booking, and refuse the wrong merchant before the charge fires.',
  },
  {
    title: 'AP & procurement agents',
    body: 'Auto-approve invoices under $1,000 against policy, and escalate the $9,300 wire for a human signature.',
  },
  {
    title: 'Account-action agents',
    body: 'Gate refunds, credits, and balance changes. Hold any refund over $200 before it hits a paying customer.',
  },
  {
    title: 'Teams adopting agent payments',
    body: 'Just turned on Visa, Stripe, or agent-commerce rails? Add $/day spend caps and a signed audit trail without building it yourself.',
  },
] as const;

export function Why() {
  return (
    <section id="use-cases" aria-labelledby="use-cases-heading" className="section">
      <Eyebrow>06 · Use cases</Eyebrow>
      <h2 id="use-cases-heading" className="section-title max-w-3xl">
        Built for teams whose agents move money.
      </h2>
      <div className="mt-12 grid gap-px bg-[var(--color-line)] md:grid-cols-2">
        {USE_CASES.map((useCase) => (
          <article key={useCase.title} className="cell p-6 md:p-7">
            <h3 className="text-xl font-semibold">{useCase.title}</h3>
            <p className="mt-4 text-sm leading-7 text-[var(--color-muted)]">{useCase.body}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
