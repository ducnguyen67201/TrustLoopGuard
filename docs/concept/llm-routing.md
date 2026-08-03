# First-party LLM routing

`config/llm-routing.json` is the single versioned authority for provider,
model, deadline, fallback, and optional reasoning-effort selection for LLM calls
owned by Featherlane AI. Credentials are deliberately outside this manifest.

This configuration is not the customer gateway route registry. Gateway
providers and models are customer-managed, durable Rust data described in
[gateway.md](gateway.md). The first-party manifest covers only models selected
by this repository for runtime judges, control-plane assistance, and bundled
demos.

## Route taxonomy

The manifest groups routes by workload so one reviewed file remains the source
of truth without forcing unlike tasks onto one model:

- Runtime judges: `hallucination`, `tone`, `authority`, and `semantic_policy`.
- Control-plane assistance: `policy_draft`, `policy_ai_edit`,
  `guardrail_generation`, and `github_integration`.
- First-party demos: `demo_default`, `demo_dispute`, and `demo_livekit`.

The committed JSON owns the exact models, deadlines, and fallbacks. Do not copy
that matrix into another document or environment variable.

## Manifest contract

The top-level `schema_version` is currently `1`. `providers` names provider
types and the environment variable containing each credential. `routes` maps a
workload name to a described primary target and optional fallback. Each target
contains:

- `provider`: a key from `providers`.
- `model`: the provider model identifier.
- `deadline_ms`: the Rust routing deadline. Demo routes retain this as metadata.
- `reasoning_effort`: optional `none`, `low`, `medium`, `high`, `xhigh`, or
  `max`.

Omitting `reasoning_effort` omits the provider request field and preserves the
provider's current behavior. All loaders reject unsupported schema versions and
reasoning values. Rust rejects unknown first-party route names, while the demo
selectors expose only their named demo workloads; none silently chooses another
model.

## Credential boundary

Provider credentials remain runtime environment variables such as
`OPENAI_API_KEY` and `OPENROUTER_API_KEY`. The manifest contains only the name
of the credential variable. It never contains a key or token.

The Rust server builds its router from the manifest embedded in `tl-llm`. A
missing provider credential disables first-party LLM routes while leaving the
server available; an invalid embedded manifest is a build/startup defect.

## Dispatch and budgets

`LlmRouter` is the only Rust provider/model dispatch point. Primary and fallback
calls share the same deadline, reasoning, error, and telemetry behavior.

Runtime Tier 3 judges use the budgeted judge API. Their completed token usage is
charged to the per-tenant `TokenBudget`, and the persisted `LlmCallAudit.judge`
contract remains unchanged. Existing policy-authoring and GitHub control-plane
operations use the generic route API. Those calls are bounded by their route
deadline but do not consume runtime judge budgets.

## Consumers and deployment

- Rust compiles the manifest into `tl-llm` with `include_str!`; no runtime path
  or mounted config file is required.
- TypeScript demos statically import the JSON so the marketing build bundles
  their route selection.
- The Python LiveKit demo reads the same checked-in JSON and selects
  `demo_livekit`.

The Rust and marketing Docker builders must copy `config/` before compiling.
Changing a route therefore requires rebuilding and redeploying the affected
artifact; there is no hot reload or environment override.

## Updating a route

1. Edit only `config/llm-routing.json` for first-party model selection.
2. Keep provider credentials in the deployment secret manager.
3. Run the manifest, provider-wire, demo-loader, and affected workload tests.
4. Rebuild and deploy the Rust server and any demo/marketing consumer affected
   by the route.
5. Verify startup logs and one bounded smoke path before removing obsolete
   deployment variables.

Model changes should be evaluated separately from routing refactors so cost,
latency, and quality changes are measurable.
