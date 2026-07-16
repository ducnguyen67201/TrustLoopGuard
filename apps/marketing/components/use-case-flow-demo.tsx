import type { ReactNode } from 'react';
import type { UseCaseDemo, UseCaseDemoEffect, UseCaseDemoField } from '@/app/use-cases/content';

const EFFECT_LABELS: Record<UseCaseDemoEffect, string> = {
  permit: 'Permit',
  transform: 'Transform',
  require_approval: 'Approval required',
  deny: 'Denied',
};

export function UseCaseFlowDemo({ demo }: { demo: UseCaseDemo }) {
  return (
    <div className="use-case-flow-demo" data-kind={demo.kind}>
      <div className="use-case-flow-demo-grid">
        <FlowStage number="01" label="Proposed action">
          <h3>{demo.proposalTitle}</h3>
          <code>{demo.proposalCode}</code>
          <FieldList fields={demo.proposalFields} />
        </FlowStage>

        <FlowStage number="02" label="Policy check">
          <h3>{demo.policyTitle}</h3>
          <FieldList fields={demo.policyFields} />
          <span className="use-case-flow-demo-active">
            <i aria-hidden="true" />
            Policy on
          </span>
        </FlowStage>

        <FlowStage number="03" label="Decision">
          <h3>Return an explicit effect</h3>
          <div className="use-case-flow-demo-decisions">
            {demo.decisions.map((decision) => (
              <div key={decision.subject} data-effect={decision.effect}>
                <span>{decision.subject}</span>
                <strong>{EFFECT_LABELS[decision.effect]}</strong>
                <small>{decision.detail}</small>
              </div>
            ))}
          </div>
        </FlowStage>

        <FlowStage number="04" label="Execution">
          <h3>{demo.executionTitle}</h3>
          <div className="use-case-flow-demo-gate" aria-hidden="true">
            <span />
            <i />
            <span />
          </div>
          <p>{demo.executionDetail}</p>
        </FlowStage>
      </div>

      <div className="use-case-flow-demo-boundary">
        <span>Execution boundary</span>
        <strong>{demo.boundary}</strong>
      </div>
    </div>
  );
}

function FlowStage({
  number,
  label,
  children,
}: {
  number: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <section className="use-case-flow-demo-stage">
      <header>
        <span>{number}</span>
        <p>{label}</p>
      </header>
      <div>{children}</div>
    </section>
  );
}

function FieldList({ fields }: { fields: readonly UseCaseDemoField[] }) {
  return (
    <dl className="use-case-flow-demo-fields">
      {fields.map((field) => (
        <div key={field.label}>
          <dt>{field.label}</dt>
          <dd>{field.value}</dd>
        </div>
      ))}
    </dl>
  );
}
