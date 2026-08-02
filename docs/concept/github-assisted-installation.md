# GitHub-Assisted Installation

GitHub-assisted installation is a Rust-owned control-plane subsystem that helps a workspace owner or admin open a draft pull request adding Featherlane AI to a selected TypeScript/Next.js repository.

The feature starts from an existing agent and environment. A user installs the separate Featherlane AI GitHub App on selected repositories, maps one repository/root to the agent, describes the irreversible action they need guarded, reviews a generated integration plan, and explicitly approves opening a draft PR.

## Ownership

Rust owns all durable state and all external GitHub writes:

- `tl-core` owns public request, response, status, proposal, and job types.
- `tl-storage` owns GitHub installation state, installations, repository connections, and integration jobs.
- `tl-server` owns GitHub App callback proof, webhook verification, repository analysis, proposal validation, worker orchestration, PR creation, and activation verification.
- `apps/web` owns only the dialog and same-origin proxy routes.

The dashboard never stores GitHub installations, repository mappings, proposals, or job state in a web database.

## Durable Entities

`github_installation_states` stores a 10-minute, single-use hash of the GitHub App setup state. The raw state only travels through the browser and GitHub.

`github_installations` stores one GitHub App installation per workspace and GitHub installation id, including account metadata, selected/all repository mode, and lifecycle status.

`github_repository_connections` maps a selected repository/root to one Featherlane AI agent and environment. The mapping carries the recipe version so future recipes can coexist.

`github_integration_jobs` stores the analysis/apply lifecycle, risk statement, proposal summary, proposed file replacements, manual steps, draft PR details, error state, and activation timestamps.

## Lifecycle

1. Owner/admin requests an install URL.
2. Rust stores a one-time state hash and returns the GitHub App setup URL.
3. GitHub redirects back to the dashboard callback.
4. Next forwards the callback to Rust with the signed-in user context.
5. Rust claims state once, verifies workspace role, exchanges the GitHub code, confirms the installation belongs to that GitHub user, and stores the installation.
6. The user selects a repository/root and maps it to an agent/environment.
7. The user provides a business-risk statement and source-processing consent.
8. Rust queues an analysis job.
9. The worker reads a bounded repository tree through GitHub, filters candidate files deterministically, sends one strict-schema LLM request, validates the proposal, and stores the reviewable plan.
10. The user approves `Open draft PR`.
11. The worker creates a deterministic branch, commit, and draft PR through the Git Data API, then redacts full proposed file contents from durable job state.
12. After merge/deploy/run, the job verifies only when a persisted trace matches workspace, environment, agent id, `featherlane_ai_integration_id`, and a post-merge timestamp.

## Security and Privacy

The GitHub App is separate from Auth.js GitHub login. Repository automation uses selected-repository GitHub App installation tokens, not dashboard OAuth login credentials.

The minimum GitHub permissions are Metadata read, Contents read/write, and Pull requests write. The App does not request Workflows, Secrets, Administration, or Checks.

Installation tokens are minted just in time, cached only in memory, and never persisted, logged, sent to the browser, or included in LLM prompts.

Repository source is bounded before it reaches the LLM. The analyzer skips environment files, workflows, lockfiles, binaries, generated/vendor directories, secret-looking paths, oversized files, and full-repository reads. The user must consent before source excerpts are processed.

Featherlane AI does not execute customer code, install dependencies, update lockfiles, run formatters, run tests, merge PRs, deploy, or create repository secrets. Customer CI and review remain authoritative.

## Supported Recipe

The first recipe is `typescript-nextjs-v1`. It targets TypeScript/Next.js repositories and inserts Featherlane AI SDK calls with this activation marker:

```ts
context: {
  featherlane_ai_integration_id: "<connection UUID>",
  featherlane_ai_recipe_version: "typescript-nextjs-v1",
}
```

That marker is not an enforcement input. It exists so the dashboard can verify the exact generated integration after the PR is merged and deployed.

## Failure Modes

Unsupported framework, truncated Git tree, unsafe root path, missing consent, unsafe proposal, stale base SHA, GitHub authorization failure, LLM unavailability, queue saturation, and repository access removal all become durable job or connection states with user-safe error messages.

Installation removal disables affected connections. Draft PR closure without merge leaves the job unverified. A merged PR does not verify until application traffic produces the exact marked trace.
