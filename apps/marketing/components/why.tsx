const USE_CASES = [
  {
    number: '01',
    title: 'Customer support',
    risk: 'The agent promises a refund or policy exception it cannot authorize.',
    control: 'Rewrite the response or escalate it with the policy reason attached.',
  },
  {
    number: '02',
    title: 'Payments & procurement',
    risk: 'A purchase, refund, or transfer exceeds the authority granted to the agent.',
    control: 'Block the action or route it to a named approver before execution.',
  },
  {
    number: '03',
    title: 'Tool-using agents',
    risk: 'Untrusted context steers an agent into a sensitive or irreversible tool call.',
    control: 'Evaluate source, provenance, tool metadata, and parameter authority together.',
  },
  {
    number: '04',
    title: 'AI gateways',
    risk: 'Unsafe input or output reaches users because enforcement lives only in prompts.',
    control: 'Apply policy at the provider boundary and attach a trace ID to the response.',
  },
] as const;

export function Why() {
  return (
    <section id="use-cases" aria-labelledby="use-cases-heading" className="section use-cases-section">
      <div className="section-heading split-heading">
        <div>
          <p className="eyebrow">Where it fits</p>
          <h2 id="use-cases-heading" className="section-title">
            The failure changes. The control point does not.
          </h2>
        </div>
        <p className="section-copy">
          Put the decision between the model and the user, tool, payment rail, or provider call
          that turns an agent's proposal into a real outcome.
        </p>
      </div>

      <div className="use-case-grid">
        {USE_CASES.map((useCase) => (
          <article key={useCase.number}>
            <header><span>{useCase.number}</span><h3>{useCase.title}</h3></header>
            <dl>
              <div><dt>Failure</dt><dd>{useCase.risk}</dd></div>
              <div><dt>Control</dt><dd>{useCase.control}</dd></div>
            </dl>
          </article>
        ))}
      </div>
    </section>
  );
}
