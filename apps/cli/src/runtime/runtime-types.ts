export type JsonPrimitive = boolean | number | string | null;
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];
export interface JsonObject {
  [key: string]: JsonValue;
}

export type HostId = 'claude' | 'codex' | 'opencode';

export interface RuntimeEnvironment {
  TLG_API_KEY?: string;
  TLG_APPROVAL_POLL_MS?: string;
  TLG_APPROVAL_TIMEOUT_MS?: string;
  TLG_REQUEST_TIMEOUT_MS?: string;
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
