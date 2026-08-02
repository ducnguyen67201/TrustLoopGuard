export type JsonPrimitive = boolean | number | string | null;
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];
export interface JsonObject {
  [key: string]: JsonValue;
}

export type HostId = 'claude' | 'codex' | 'opencode';

export interface RuntimeEnvironment {
  FEATHERLANE_AI_API_KEY?: string;
  FEATHERLANE_AI_APPROVAL_POLL_MS?: string;
  FEATHERLANE_AI_APPROVAL_TIMEOUT_MS?: string;
  FEATHERLANE_AI_REQUEST_TIMEOUT_MS?: string;
}

export interface RuntimeRegistration {
  root: string;
  url: string;
  agentId: string;
  targets: HostId[];
  cliVersion: string;
  runtimeVersion: string;
  createdAt: string;
  updatedAt: string;
}
