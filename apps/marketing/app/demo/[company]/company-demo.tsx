'use client';

import type { CSSProperties } from 'react';
import { useState } from 'react';

import type { CompanyDemoViewModel, DemoEffect } from '../company-profile';
import styles from '../demo.module.css';

type CompanyDemoProps = {
  profile: CompanyDemoViewModel;
};

type CompanyBrandStyle = CSSProperties & {
  '--company-accent': string;
  '--company-accent-soft': string;
};

const effectLabels: Record<DemoEffect, string> = {
  permit: 'Allowed',
  require_approval: 'Approval required',
  deny: 'Blocked',
};

const effectClasses: Record<DemoEffect, string> = {
  permit: styles['executed'] ?? '',
  require_approval: styles['held'] ?? '',
  deny: styles['blocked'] ?? '',
};

export function CompanyDemo({ profile }: CompanyDemoProps) {
  const [selectedEffect, setSelectedEffect] = useState<DemoEffect>('require_approval');
  const selectedPath =
    profile.paths.find(({ effect }) => effect === selectedEffect) ?? profile.paths[0];
  const companyInitial = profile.company_name.slice(0, 1).toUpperCase();
  const brandStyle: CompanyBrandStyle = {
    '--company-accent': profile.branding.primary_color,
    '--company-accent-soft': profile.branding.secondary_color,
  };

  return (
    <div className={styles['companyDemo']} style={brandStyle}>
      <section className={styles['shell']} aria-label={`${profile.company_name} concept demo`}>
        <div className={styles['chatPanel']}>
          <header className={styles['panelHeader']}>
            <div>
              <p>{profile.user_profile}</p>
              <h2>{profile.workflow}</h2>
            </div>
            <span className={styles['companyMark']} aria-hidden="true">
              {companyInitial}
            </span>
          </header>

          <div className={styles['chatBody']} aria-live="polite">
            <article className={styles['assistantMessage']}>
              <span>Agent workflow</span>
              <p>{profile.risk_boundary}</p>
            </article>
            <article className={styles['customerMessage']}>
              <span>Proposed action</span>
              <p>{selectedPath.proposal}</p>
            </article>
            <article className={styles['evidenceCard']}>
              <span>Evidence considered</span>
              <ul>
                {selectedPath.evidence.map((item) => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </article>
          </div>

          <div className={styles['conceptControls']}>
            <span>Choose a decision path</span>
            <div className={styles['pathButtons']}>
              {profile.paths.map((path) => (
                <button
                  key={path.effect}
                  type="button"
                  className={path.effect === selectedEffect ? styles['selectedPath'] : undefined}
                  aria-pressed={path.effect === selectedEffect}
                  onClick={() => setSelectedEffect(path.effect)}
                >
                  {path.label}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className={styles['controlPanel']}>
          <header className={styles['panelHeader']}>
            <div>
              <p>Execution trace</p>
              <h2>The control boundary</h2>
            </div>
            <span className={`${styles['decisionBadge']} ${effectClasses[selectedPath.effect]}`}>
              {effectLabels[selectedPath.effect]}
            </span>
          </header>

          <div className={styles['workflow']}>
            <article className={`${styles['workflowStep']} ${styles['complete']}`}>
              <span>01</span>
              <div>
                <h3>Agent proposal</h3>
                <p>{selectedPath.proposal}</p>
              </div>
              <i aria-hidden="true" />
            </article>
            <article className={`${styles['workflowStep']} ${styles['complete']}`}>
              <span>02</span>
              <div>
                <h3>Workflow evidence</h3>
                <p>Checks the public-source facts represented in this scenario.</p>
              </div>
              <i aria-hidden="true" />
            </article>
            <article
              className={`${styles['workflowStep']} ${styles['guardStep']} ${styles['complete']}`}
            >
              <span>03</span>
              <div>
                <h3>TrustLoopGuard</h3>
                <p>{profile.rule}</p>
              </div>
              <i aria-hidden="true" />
            </article>
            <article className={`${styles['workflowStep']} ${styles['complete']}`}>
              <span>04</span>
              <div>
                <h3>{effectLabels[selectedPath.effect]}</h3>
                <p>{selectedPath.decision}</p>
              </div>
              <i aria-hidden="true" />
            </article>
          </div>

          <div className={`${styles['proofGrid']} ${styles['companyProofGrid']}`}>
            <article>
              <span>Policy boundary</span>
              <strong>{profile.scenario_id}</strong>
              <p>{profile.rule}</p>
            </article>
            <article>
              <span>Approval step</span>
              <strong>Human control</strong>
              <p>{profile.approval_step}</p>
            </article>
            <article>
              <span>Decision record</span>
              <strong>Auditable proof</strong>
              <p>{profile.record_shown}</p>
            </article>
          </div>

          <footer className={styles['conceptFooter']}>
            <p>{profile.disclaimer}</p>
            <div>
              {profile.sources.map((source) => (
                <a key={source.url} href={source.url} target="_blank" rel="noreferrer">
                  {source.title} <span aria-hidden="true">↗</span>
                </a>
              ))}
            </div>
          </footer>
        </div>
      </section>
    </div>
  );
}
