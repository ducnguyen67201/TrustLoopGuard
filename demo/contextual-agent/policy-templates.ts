import type { Severity } from '@featherlane-ai/sdk';

import type { ContextualPolicyPhase } from './config';

export interface ContextualPolicyTemplate {
  id: string;
  phase: ContextualPolicyPhase;
  description: string;
  severity: Severity;
  action: 'permit' | 'deny' | 'defer' | 'transform';
  source: string;
}

/** Setup-only desired state. Runtime inventory and decisions always come from Rust. */
export const CONTEXTUAL_POLICY_TEMPLATES: readonly ContextualPolicyTemplate[] = [
  {
    id: 'contextual-readonly-input',
    phase: 'input',
    description: 'Recognize read-only questions that stay inside the synthetic workflow scope.',
    severity: 'low',
    action: 'permit',
    source: `id: contextual-readonly-input
description: Recognize read-only questions that stay inside the synthetic workflow scope.
severity: low
owner_agent_id: contextual-demo-agent
when:
  agents: [contextual-demo-agent]
  channels: [chat]
  domains: [contextual_agent_action_input]
match:
  any:
    - regex: '(?i)\\b(read[- ]?only|inspect|summari[sz]e|explain|show status|check health|what (?:can|would))\\b'
    - semantic: 'The user asks for explanation, inspection, status, or another read-only action inside the described synthetic workflow.'
action: permit
`,
  },
  {
    id: 'contextual-shared-change-input',
    phase: 'input',
    description: 'Hold requests that would change shared configuration for human approval.',
    severity: 'high',
    action: 'defer',
    source: `id: contextual-shared-change-input
description: Hold requests that would change shared configuration for human approval.
severity: high
owner_agent_id: contextual-demo-agent
when:
  agents: [contextual-demo-agent]
  channels: [chat]
  domains: [contextual_agent_action_input]
match:
  any:
    - regex: '(?i)\\b(change|update|modify|delete|disable|rotate|grant|revoke)\\b.{0,100}\\b(shared|production|retention|permission|access|credential|configuration|setting)\\b'
    - semantic: 'The user asks the agent to make a state-changing or shared-system change that should require a human approval gate.'
action: defer
`,
  },
  {
    id: 'contextual-control-bypass-input',
    phase: 'input',
    description: 'Block attempts to bypass controls or use a human credential.',
    severity: 'critical',
    action: 'deny',
    source: `id: contextual-control-bypass-input
description: Block attempts to bypass controls or use a human credential.
severity: critical
owner_agent_id: contextual-demo-agent
when:
  agents: [contextual-demo-agent]
  channels: [chat]
  domains: [contextual_agent_action_input]
match:
  any:
    - regex: '(?i)\\b(ignore|bypass|skip|disable)\\b.{0,80}\\b(policy|policies|approval|guard|control|rule|rules)\\b'
    - regex: '(?i)\\b(human|personal|administrator|admin)\\b.{0,40}\\b(password|credential|token|api key|secret)\\b'
    - semantic: 'The user asks to bypass authorization, suppress a guardrail, impersonate a person, or use a human credential or secret.'
action: deny
`,
  },
  {
    id: 'contextual-secret-output',
    phase: 'output',
    description: 'Block assistant drafts that expose credentials, tokens, or private records.',
    severity: 'critical',
    action: 'deny',
    source: `id: contextual-secret-output
description: Block assistant drafts that expose credentials, tokens, or private records.
severity: critical
owner_agent_id: contextual-demo-agent
when:
  agents: [contextual-demo-agent]
  channels: [chat]
  domains: [contextual_agent_action_output]
match:
  any:
    - regex: '(?i)\\b(api[_ -]?key|password|secret|access[_ -]?token)\\b\\s*[:=]\\s*[A-Za-z0-9_\\-]{8,}'
    - semantic: 'The assistant draft reveals or requests a password, API key, token, private record, or human credential.'
action: deny
`,
  },
  {
    id: 'contextual-false-execution-output',
    phase: 'output',
    description: 'Replace unsupported claims that the concept accessed or changed a real system.',
    severity: 'high',
    action: 'transform',
    source: `id: contextual-false-execution-output
description: Replace unsupported claims that the concept accessed or changed a real system.
severity: high
owner_agent_id: contextual-demo-agent
when:
  agents: [contextual-demo-agent]
  channels: [chat]
  domains: [contextual_agent_action_output]
match:
  any:
    - regex: '(?i)\\b(i|we) (changed|updated|deleted|disabled|rotated|granted|revoked|accessed|inspected)\\b.{0,100}\\b(system|account|bucket|configuration|setting|credential|permission)\\b'
    - regex: '(?i)\\b(inspecting|checked|confirmed|found|retrieved)\\b.{0,120}\\b(system|account|storage|bucket|node|configuration|status|alert|log|metric|performance)\\b'
    - regex: '(?i)\\b(system|account|storage|bucket|node|configuration|status|alert|log|metric|performance)\\b.{0,60}\\b(shows?|reports?|has|is|are)\\b.{0,80}\\b(no |normal|healthy|active|degraded|pending|current)\\b'
    - semantic: 'The assistant claims it accessed, inspected, modified, approved, or executed an action in a real company system.'
action: transform
rewrite: 'This is a synthetic concept. No real system was accessed or changed. I can explain the proposed control decision and the human approval step.'
`,
  },
];
