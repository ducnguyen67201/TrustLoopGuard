export type JsonPrimitive = boolean | number | string | null;
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];
export interface JsonObject {
  [key: string]: JsonValue;
}

export const HOST_IDS = ['claude', 'codex', 'opencode'] as const;
export type HostId = (typeof HOST_IDS)[number];
export type TargetSelection = HostId[] | 'auto' | 'all';

export interface ProjectRegistration {
  root: string;
  url: string;
  agentId: string;
  targets: HostId[];
  cliVersion: string;
  runtimeVersion: string;
  createdAt: string;
  updatedAt: string;
}

export interface Registry {
  version: 1;
  projects: ProjectRegistration[];
}

export interface CliEnvironment {
  APPDATA?: string;
  CODEX_HOME?: string;
  HOME?: string;
  PATH?: string;
  PATHEXT?: string;
  TLG_AGENT_ID?: string;
  TLG_API_KEY?: string;
  TLG_APPROVAL_POLL_MS?: string;
  TLG_APPROVAL_TIMEOUT_MS?: string;
  TLG_REQUEST_TIMEOUT_MS?: string;
  TLG_URL?: string;
  XDG_CONFIG_HOME?: string;
}

interface BaseCommandOptions {
  project: string;
  json: boolean;
}

export interface InstallCommandOptions extends BaseCommandOptions {
  command: 'install';
  agentId: string | undefined;
  allowUnsupported: boolean;
  target: TargetSelection;
  url: string | undefined;
}

export interface StatusCommandOptions extends BaseCommandOptions {
  command: 'status';
}

export interface DoctorCommandOptions extends BaseCommandOptions {
  command: 'doctor';
}

export interface UninstallCommandOptions extends BaseCommandOptions {
  command: 'uninstall';
  all: boolean;
  target: TargetSelection;
}

export interface HelpCommandOptions {
  command: 'help';
}

export type CliCommandOptions =
  | DoctorCommandOptions
  | HelpCommandOptions
  | InstallCommandOptions
  | StatusCommandOptions
  | UninstallCommandOptions;

export type Compatibility = 'supported' | 'unsupported' | 'unknown';
export type Activation = 'active' | 'trust_required' | 'configured' | 'inactive' | 'unknown';
export type Coverage = 'universal' | 'host_emitted_only' | 'none';

export interface HostDetection {
  found: boolean;
  version: string | null;
  compatibility: Compatibility;
  executable: string;
}

export interface HostStatus {
  id: HostId;
  installed: boolean;
  runtimePresent: boolean;
  version: string | null;
  compatibility: Compatibility;
  activation: Activation;
  coverage: Coverage;
  exceptions: string[];
  remediation: string | null;
}

export interface CommandContext {
  cwd: string;
  env: CliEnvironment;
  homeDirectory?: string;
  platform?: NodeJS.Platform;
  runtimeSourceDirectory?: string;
  stdout: (message: string) => void;
  stderr: (message: string) => void;
}

export class CliError extends Error {
  readonly exitCode: 1 | 2 | 3;
  readonly remediation: string | undefined;

  constructor(message: string, exitCode: 1 | 2 | 3 = 1, remediation?: string) {
    super(message);
    this.name = 'CliError';
    this.exitCode = exitCode;
    this.remediation = remediation;
  }
}
