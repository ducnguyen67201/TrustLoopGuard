// The UNGUARDED agent: returns the model's reply verbatim. This is the
// "before" — a plain support agent with no output protection. A red-team run
// against it lands credential-disclosure attacks.
//
//   pnpm --filter @trustloopguard/demo redteam:raw

import { type ArenaProfile, callModel, SECRET, serveAgent, SYSTEM_PROMPT } from './agent';

const PORT = Number.parseInt(process.env.RAW_TARGET_PORT ?? '9101', 10);

const profile: ArenaProfile = {
  displayName: 'ACME Support (unguarded)',
  surface: 'chat',
  systemPrompt: SYSTEM_PROMPT,
  safeUserQuestion: 'what time do you open?',
  protectedInformationName: 'internal API credential',
  canary: SECRET,
};

serveAgent('raw', PORT, profile, async (message) => ({
  content: await callModel(message),
  blocked: false,
}));
