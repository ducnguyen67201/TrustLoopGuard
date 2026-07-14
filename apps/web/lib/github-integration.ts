'use client';

import { z } from 'zod';

const repositorySchema = z.object({
  repository_id: z.string(),
  owner: z.string(),
  name: z.string(),
  full_name: z.string(),
  default_branch: z.string(),
  private: z.boolean(),
  archived: z.boolean(),
  connected: z.boolean().default(false),
});

const connectionSchema = z.object({
  id: z.string(),
  repository_id: z.string(),
  owner: z.string(),
  name: z.string(),
  default_branch: z.string(),
  root_path: z.string(),
  agent_id: z.string(),
  environment_id: z.string(),
  status: z.string(),
  recipe_version: z.string(),
});

const proposedChangeSchema = z.object({
  path: z.string(),
  operation: z.enum(['create', 'update']),
  content_sha: z.string(),
  replacement: z.string(),
  rationale: z.string(),
});

const manualStepSchema = z.object({
  label: z.string(),
  command: z.string(),
  reason: z.string(),
});

const jobSchema = z.object({
  id: z.string(),
  connection_id: z.string(),
  status: z.string(),
  risk_statement: z.string(),
  analysis_summary: z
    .object({
      detected_framework: z.string(),
      package_manager: z.string(),
      summary: z.string(),
      integration_points: z.array(z.string()).default([]),
    })
    .nullable()
    .optional(),
  proposed_changes: z.array(proposedChangeSchema).default([]),
  manual_steps: z.array(manualStepSchema).default([]),
  pull_request_url: z.string().nullable().optional(),
  error_code: z.string().nullable().optional(),
  error_message: z.string().nullable().optional(),
  first_verified_trace_at: z.string().nullable().optional(),
});

export type GitHubRepository = z.infer<typeof repositorySchema>;
export type GitHubConnection = z.infer<typeof connectionSchema>;
export type GitHubIntegrationJob = z.infer<typeof jobSchema>;

export async function createInstallUrl(): Promise<string> {
  const res = await fetch('/api/github-integration/install-url', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({}),
  });
  const data = await readJson(res);
  return z.object({ install_url: z.string().url() }).parse(data).install_url;
}

export async function listRepositories(): Promise<GitHubRepository[]> {
  const res = await fetch('/api/github-integration/repositories');
  const data = await readJson(res);
  return z.object({ repositories: z.array(repositorySchema) }).parse(data).repositories;
}

export async function listConnections(agentId: string): Promise<GitHubConnection[]> {
  const params = new URLSearchParams({ agent_id: agentId });
  const res = await fetch(`/api/github-integration/connections?${params}`);
  const data = await readJson(res);
  return z.object({ connections: z.array(connectionSchema) }).parse(data).connections;
}

export async function createConnection(input: {
  repositoryId: string;
  rootPath: string;
  agentId: string;
  environmentId: string;
}): Promise<GitHubConnection> {
  const res = await fetch('/api/github-integration/connections', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      repository_id: input.repositoryId,
      root_path: input.rootPath,
      agent_id: input.agentId,
      environment_id: input.environmentId,
    }),
  });
  return connectionSchema.parse(await readJson(res));
}

export async function createJob(input: {
  connectionId: string;
  riskStatement: string;
  sourceProcessingConsent: boolean;
}): Promise<GitHubIntegrationJob> {
  const res = await fetch('/api/github-integration/jobs', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      connection_id: input.connectionId,
      risk_statement: input.riskStatement,
      source_processing_consent: input.sourceProcessingConsent,
    }),
  });
  return jobSchema.parse(await readJson(res));
}

export async function getJob(jobId: string, signal?: AbortSignal): Promise<GitHubIntegrationJob> {
  const init: RequestInit = signal ? { signal } : {};
  const res = await fetch(`/api/github-integration/jobs/${encodeURIComponent(jobId)}`, init);
  return jobSchema.parse(await readJson(res));
}

export async function approveJob(jobId: string): Promise<GitHubIntegrationJob> {
  const res = await fetch(`/api/github-integration/jobs/${encodeURIComponent(jobId)}/approve`, {
    method: 'POST',
  });
  const data = await readJson(res);
  return jobSchema.parse(z.object({ job: jobSchema }).parse(data).job);
}

async function readJson(res: Response): Promise<unknown> {
  const data: unknown = await res.json().catch(() => ({}));
  if (!res.ok) {
    const message =
      data !== null &&
      typeof data === 'object' &&
      'message' in data &&
      typeof data.message === 'string'
        ? data.message
        : data !== null &&
            typeof data === 'object' &&
            'error' in data &&
            typeof data.error === 'string'
          ? data.error
          : 'GitHub integration request failed';
    throw new Error(message);
  }
  return data;
}
