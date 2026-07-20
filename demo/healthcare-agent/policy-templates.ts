import type { Severity } from '@trustloopguard/sdk';

export type HealthcarePolicyPhase = 'input' | 'output';

export interface HealthcarePolicyTemplateSummary {
  description: string;
  severity: Severity;
  action: 'deny' | 'transform';
  phase: HealthcarePolicyPhase;
}

export interface HealthcarePolicyTemplate {
  id: string;
  summary: HealthcarePolicyTemplateSummary;
  source: string;
}

/** Setup-only desired state. Runtime policy inventory always comes from Rust. */
export const HEALTHCARE_POLICY_TEMPLATES: readonly HealthcarePolicyTemplate[] = [
  {
    id: 'healthcare-emergency-input',
    summary: {
      description: 'Escalate emergency symptoms before model generation.',
      severity: 'critical',
      action: 'deny',
      phase: 'input',
    },
    source: `id: healthcare-emergency-input
description: Escalate emergency symptoms before model generation.
severity: critical
owner_agent_id: healthcare-demo-agent
when:
  agents: [healthcare-demo-agent]
  channels: [chat]
  domains: [healthcare_input]
match:
  any:
    - regex: '(?i)\\b(chest pain|trouble breathing|difficulty breathing|stroke|suicidal|suicide|self[- ]?harm|severe bleeding|đau ngực|khó thở|đột quỵ|tự tử|tự làm hại|chảy máu (?:nặng|nghiêm trọng))\\b'
    - semantic: 'The user describes symptoms or intent that may require immediate emergency help.'
action: deny
`,
  },
  {
    id: 'healthcare-clinical-advice-input',
    summary: {
      description: 'Keep the scheduling agent out of diagnosis and prescribing.',
      severity: 'high',
      action: 'deny',
      phase: 'input',
    },
    source: `id: healthcare-clinical-advice-input
description: Keep the scheduling agent out of diagnosis and prescribing.
severity: high
owner_agent_id: healthcare-demo-agent
when:
  agents: [healthcare-demo-agent]
  channels: [chat]
  domains: [healthcare_input]
match:
  any:
    - regex: '(?i)\\b(diagnos(?:e|is)|double (?:my )?(?:insulin )?dose|stop (?:taking )?(?:my )?medication|change (?:my )?medication|chẩn đoán|tăng gấp đôi (?:liều )?(?:insulin)?|ngừng (?:uống )?thuốc|đổi thuốc|thay đổi thuốc)\\b'
    - semantic: 'The user asks for diagnosis, treatment, prescribing, dosage, or medication-change advice.'
action: deny
`,
  },
  {
    id: 'healthcare-other-patient-data-input',
    summary: {
      description: "Refuse requests for another person's health information.",
      severity: 'critical',
      action: 'deny',
      phase: 'input',
    },
    source: `id: healthcare-other-patient-data-input
description: Refuse requests for another person's health information.
severity: critical
owner_agent_id: healthcare-demo-agent
when:
  agents: [healthcare-demo-agent]
  channels: [chat]
  domains: [healthcare_input]
match:
  any:
    - regex: '(?i)(?:\\b(?:another|other) patient(?:''s)?\\b.{0,80}\\b(?:record|diagnosis|appointment|details?)\\b|\\b(?:bệnh nhân|người bệnh) khác\\b.{0,80}\\b(?:hồ sơ|chẩn đoán|cuộc hẹn|thông tin)\\b|\\b(?:hồ sơ|chẩn đoán|cuộc hẹn|thông tin)\\b.{0,80}\\b(?:bệnh nhân|người bệnh) khác\\b)'
    - semantic: 'The user requests health, identity, record, or appointment information about another person.'
action: deny
`,
  },
  {
    id: 'healthcare-clinical-advice-output',
    summary: {
      description: 'Replace clinical recommendations with a clinician handoff.',
      severity: 'high',
      action: 'transform',
      phase: 'output',
    },
    source: `id: healthcare-clinical-advice-output
description: Replace clinical recommendations with a clinician handoff.
severity: high
owner_agent_id: healthcare-demo-agent
when:
  agents: [healthcare-demo-agent]
  channels: [chat]
  domains: [healthcare_output]
match:
  any:
    - regex: '(?i)(?:\\byou should (?:take|stop|double|increase|decrease)\\b|\\bbạn nên (?:uống|ngừng|tăng|giảm|dùng|gấp đôi)\\b)'
    - semantic: 'The assistant draft diagnoses, prescribes, recommends treatment, or changes medication.'
action: transform
rewrite: "I can help schedule a visit, but I can't diagnose symptoms or recommend medication changes. Please contact a licensed clinician."
`,
  },
  {
    id: 'healthcare-identifier-output',
    summary: {
      description: 'Suppress patient identifiers and unverified record disclosure.',
      severity: 'critical',
      action: 'deny',
      phase: 'output',
    },
    source: `id: healthcare-identifier-output
description: Suppress patient identifiers and unverified record disclosure.
severity: critical
owner_agent_id: healthcare-demo-agent
when:
  agents: [healthcare-demo-agent]
  channels: [chat]
  domains: [healthcare_output]
match:
  any:
    - regex: '(?i)\\b(?:ssn|social security number)\\b\\s*[:#-]?\\s*\\d{3}-?\\d{2}-?\\d{4}\\b'
    - regex: '(?i)\\bmrn\\b\\s*[:#-]?\\s*[a-z0-9-]{6,20}\\b'
    - semantic: 'The assistant draft reveals patient identity, records, appointments, or health information without verified authorization.'
action: deny
`,
  },
  {
    id: 'healthcare-respectful-output',
    summary: {
      description: 'Replace demeaning or dismissive healthcare replies with a calm handoff.',
      severity: 'high',
      action: 'transform',
      phase: 'output',
    },
    source: `id: healthcare-respectful-output
description: Replace demeaning or dismissive healthcare replies with a calm handoff.
severity: high
owner_agent_id: healthcare-demo-agent
when:
  agents: [healthcare-demo-agent]
  channels: [chat]
  domains: [healthcare_output]
match:
  semantic: 'The assistant draft insults, dismisses, shames, blames, or demeans the user.'
action: transform
rewrite: 'A hospital teammate needs to review this before we continue.'
`,
  },
];
