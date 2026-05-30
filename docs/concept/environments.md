# Environments

Environments are workspace-owned runtime and deployment boundaries. They let a workspace run the same agent and policy definitions in `dev`, `staging`, and `production` without cloning agents or mutating production policy deployment state.

## Ownership

Rust owns environments and their deployment state.

```text
Browser dashboard
  -> Next.js same-origin proxy
  -> Rust /v1/environments
  -> tl-storage workspace_environments
```

The web dashboard may select an environment and pass it to same-origin API routes. It must not store environment state or decide runtime policy deployment itself.

## Relationship to Workspaces

Each workspace has one or more environments. Existing workspaces are migrated with a default `production` environment.
Legacy runtime rows whose `workspace_id` does not exist in `workspaces` are removed during the environment migration
instead of being preserved as synthetic workspaces.

```text
workspace
  -> environments
  -> agents
  -> policy definitions

environment
  -> runtime API keys
  -> policy deployments
  -> runs
  -> traces
  -> analytics scope
```

Agents and policy definitions stay workspace-level. A policy deployment is the environment-specific row that says whether a policy is enabled in that environment and which version is deployed.

Workspace creation seeds disabled starter policy definitions for common PII and prompt-injection patterns. Those starter policies are ordinary workspace policies: each environment starts with them disabled, and users decide whether to enable, edit, or delete them.

## Runtime Resolution

Runtime SDK and gateway calls resolve their environment from the `workspace_api_keys.environment_id` row. Callers cannot override that environment in the request body or with a header.

Dashboard/internal calls may pass an explicit selected environment through trusted same-origin proxy context. If no environment is selected, Rust defaults to the workspace default environment.

## Policy Deployment

`policies.enabled` is not the runtime source of truth. Runtime checks load enabled policies through `policy_environment_deployments` for the resolved `(workspace_id, environment_id)` pair, then apply normal policy matching by agent, channel, domain, and matcher.

This means the same `agent_id` can enforce stricter policies in `dev` than in `production`, or test a draft policy in `dev` while production continues using its existing deployment set.

## API

Environment management is exposed through Rust:

- `GET /v1/environments`
- `POST /v1/environments`
- `PATCH /v1/environments/{id}`
- `DELETE /v1/environments/{id}`

Deletion is a soft delete and is blocked when durable runtime data still references the environment.
