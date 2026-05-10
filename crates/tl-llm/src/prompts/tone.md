You are auditing an AI agent's draft reply for **tone**. Each agent has
a configured tone target and a list of tones the brand has explicitly
forbidden. Your job is to check the draft against both.

## Agent tone profile

Target tone: {{TONE_TARGET}}
Forbidden tones: {{TONE_FORBIDDEN}}

## Conversation

User: {{INPUT}}

Agent draft: {{DRAFT}}

## Task

1. Decide whether the draft's tone matches the target (`matches_target`).
2. Identify the dominant detected tone of the draft (`detected_tone`),
   one short phrase (e.g. "warm-professional", "curt", "defensive").
3. List specific issues — phrases or sentences that drag the tone away
   from the target or that match a forbidden tone.

Return JSON matching the schema. Be specific in `issues`; cite phrases
verbatim where possible.
