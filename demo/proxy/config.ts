import { buildChatBreakCases, type ChatBreakCase } from '../agent-breaker/index';
import { proxySupportAgent } from './agent-profile';

export { proxyAgentSystemPrompt, proxySupportAgent } from './agent-profile';

export interface GatewayResourceIds {
  agent: string;
  provider: string;
  profile: string;
  route: string;
}

export const proxyDemoConfig = {
  providerSecret: 'provider-secret',
  cleanProviderReply: "We're open 9 am to 5 pm on weekdays.",
  unsafeProviderReply: 'unsafe proxy reply',
  fallbackMessage: 'Blocked by TrustLoopGuard proxy demo.',
};

export const chatBreakCases = buildChatBreakCases(proxySupportAgent);

export function mockProviderReplyFor(caseUnderTest: ChatBreakCase): string {
  if (caseUnderTest.expect === 'blocked_output') {
    return proxyDemoConfig.unsafeProviderReply;
  }

  return proxyDemoConfig.cleanProviderReply;
}

export function createGatewayResourceIds(runId: string): GatewayResourceIds {
  return {
    agent: `demo-proxy-agent-${runId}`,
    provider: `demo-proxy-provider-${runId}`,
    profile: `demo-proxy-profile-${runId}`,
    route: `demo-proxy-route-${runId}`,
  };
}

export function blockingPolicyYaml(runId: string, ids: GatewayResourceIds): string {
  return `
id: demo-proxy-block-output-${runId}
description: Block the mock provider output in the proxy demo.
when:
  channels: [chat]
match:
  literal: ${JSON.stringify(proxyDemoConfig.unsafeProviderReply)}
action: block
owner_agent_id: ${ids.agent}
`.trim();
}

export function providerConnectionPayload(ids: GatewayResourceIds, providerUrl: string) {
  return {
    id: ids.provider,
    display_name: 'Proxy demo mock OpenAI provider',
    kind: 'openai_compatible',
    base_url: providerUrl,
    default_model: proxySupportAgent.model,
    provider_api_key: proxyDemoConfig.providerSecret,
  };
}

export function enforcementProfilePayload(ids: GatewayResourceIds) {
  return {
    id: ids.profile,
    display_name: 'Proxy demo strict output',
    input_action: 'allow',
    output_action: 'block',
    fail_mode: 'closed',
    retention_mode: 'metadata_only',
    fallback_message: proxyDemoConfig.fallbackMessage,
    max_regenerations: 0,
  };
}

export function gatewayRoutePayload(ids: GatewayResourceIds) {
  return {
    id: ids.route,
    display_name: 'Proxy demo route',
    provider_connection_id: ids.provider,
    agent_id: ids.agent,
    enforcement_profile_id: ids.profile,
  };
}
