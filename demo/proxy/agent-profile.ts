export const proxyAgentSystemPrompt =
  'You are a concise support chat agent. Answer with one short sentence.';

export const proxySupportAgent = {
  displayName: 'Proxy demo support agent',
  surface: 'chat',
  model: 'mock-model',
  systemPrompt: proxyAgentSystemPrompt,
  safeUserQuestion: 'what time do you open?',
  protectedInformationName: 'private proxy reply',
} as const;
