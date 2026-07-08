# Graph Report - TrustLoopGuard  (2026-07-08)

## Corpus Check
- 1336 files · ~654,440 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 14204 nodes · 29009 edges · 1833 communities (1185 shown, 648 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 1617 edges (avg confidence: 0.71)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `9bc0c353`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Client
- GuardEvent
- cn
- AnalyticsCatalogDimension
- fetchMock
- Enum
- oauth.rs
- FinancialActionsContent.tsx
- dashboard-data.ts
- GatewayPageContent.tsx
- Integrating TrustLoopGuard
- code:block1 (POST /v1/check)
- proxyRustJson
- Field-by-field
- code:yaml (id: refund-guarantee)
- PolicyEditorDialog.tsx
- ApiErrorCode
- redteam.rs
- settings_update.rs
- types.py
- tests.rs
- Client
- errors.ts
- AgentListResponse
- RunSummary
- code:block1 (tl-cli      tl-server      tl-sdk-rust)
- 0. Start the server (all languages need this)
- code:block1 (Guard.check(draft, ctx) -> Decision)
- githubRepo
- TrustLoopGuard demos
- Result
- AnalyticsChartGrid.tsx
- param_auth.rs
- PostgresGatewayAdapter
- code:bash (curl -X PATCH \)
- llm_pricing.rs
- latest_review_outcomes
- SDK-Driven Development at TrustLoopGuard
- code:text (id: id must use lowercase letters, numbers, '-' or '_')
- code:python (import trustloopguard as trustloop)
- tests.rs
- create_knowledge_source
- label.rs
- BadgeProps
- auth.rs
- Ownership
- code:block1 (+-------------------+      CheckRequest       +-------------)
- Domain terms
- RunRepo
- agent.rs
- tests.rs
- apiKeyHeaders
- scenarios.core.ts
- report.rs
- profile_record_to_wire
- AsyncClient
- Gateway Proxy Runtime Branch Guide
- EnvironmentStoreError
- properties
- code:text (policies/refund-promise.yaml)
- GuardEvent.ts
- rustApiForAuthorizedWorkspace
- scripts
- guard.ts
- tlClientForRequest
- errorResponse
- PolicyState
- models.rs
- RunDetailLiveView.tsx
- synthesis.rs
- properties
- JwtSigner
- mod.rs
- AgentRepo
- AuthConfig
- RunnerError
- Load test
- Default
- change_password
- pipeline_e2e.rs
- schema.rs
- attacks-panel.tsx
- RedteamState
- gateway.rs
- WorkspaceKeyContext
- run-detail-live.ts
- share.rs
- checker_enforcement.rs
- CheckerFinding
- FinancialStoreError
- path
- Policy
- normalization.rs
- event_ingestion.rs
- SdkError
- AnalyticsStoreError
- redteam-report.ts
- tests.rs
- req
- payload
- create_api_key
- AppState
- AnalyticsDashboardWidget.ts
- RedteamReportShareRepo
- TraceStoreError
- Technical terms
- tool-runner.ts
- code:text (agent drafts risky output)
- dashboard_admin_repo.rs
- PolicyStoreError
- financial_authorization_service.rs
- family_parse.rs
- BudgetAlertRepo
- analytics.rs
- index.ts
- HnswFuzzyChecker
- policy_cli.rs
- tl-client.ts
- adapter.ts
- @auth/drizzle-adapter
- compilerOptions
- { GET, POST }
- README.md
- PostgresRedteamJobAdapter
- FinancialActionRecord
- event_policy.rs
- server.ts
- labels.rs
- code:bash (npm view @trustloopguard/sdk version)
- api_error_response
- package.json
- PoliciesPageContent.tsx
- RedteamJobStoreError
- gateway_budget.rs
- FinancialActionDecisionReceipt.ts
- seo-landing-page.tsx
- SAMPLES
- properties
- Result
- MemoryFinancialStore
- properties
- type
- LabelPolicyProvider
- Repository Agent Instructions
- latest_review_outcomes
- ToolMetadataRepo
- TierResult
- tier_results
- code:block1 (agent proposes output → trustloop.check(...) → allow | block)
- v0 Design Decisions
- Runtime Refactor Jobs
- agents
- PostgresFinancialAdapter
- in_scope
- properties
- definitions
- Dashboard
- package.json
- AgentTone
- route.ts
- definitions
- TriggeredPolicy
- Acceptance flow (Option A)
- proxy-helpers.ts
- financial.rs
- UserRepo
- drizzle-kit
- db:generate
- pull_request_template.md
- WorkspaceInvite.ts
- validation.rs
- definitions
- default
- PolicyRepo
- workflow_requirements
- compilerOptions
- channels
- policy.schema.json
- lint-web-backend-only.sh
- Client
- code:text (┌─────────────────────────────────────┐)
- lint-no-internal-imports.sh
- FamilyPolicy
- budget_alerts.rs
- PostgresPolicyAdapter
- MemoryRunStore
- code:text (UI component)
- RedteamJobStore
- lint-storage-boundaries.sh
- CheckerRun.ts
- lint-api-contracts.sh
- validation.rs
- dashboard.rs
- policy.rs
- redteam_runner.rs
- check-schema-drift.sh
- PostgresAnalyticsAdapter
- value_limit.rs
- metrics.rs
- RunState
- tool_metadata.rs
- EscalationRepo
- RouterConfig
- package.json
- entrypoint
- GatewayStoreError
- code:text (Browser / SDK)
- engine.rs
- code:ts (const decision = await client.check({)
- README.md
- code:text (Customer app -> SDK -> /v1/check -> Decision -> customer han)
- plan.rs
- escalation.rs
- TeamStoreError
- RetryConfig
- KnowledgeRepo
- MemoryAnalyticsStore
- writer.rs
- package.json
- run_summary
- DashboardAdminStoreError
- Result
- .prettierrc.json
- dependencies
- MokaCache
- check.ts
- lib.rs
- code:text (app -> /v1/gateway/<route_id>/openai -> TrustLoopGuard -> pr)
- page.tsx
- event
- tool.rs
- finalize_gateway_response
- label_policy.rs
- code:text (source of truth)
- hero.tsx
- code:text (Dashboard / customer integration)
- properties
- code:bash (npm install @trustloopguard/sdk)
- harden_job
- gateway.rs
- writer.rs
- memory.rs
- traces.rs
- EnvironmentRepo
- retry.rs
- Contributing to TrustLoopGuard
- RedteamPlanRepo
- knowledge.rs
- precommit-typecheck.sh
- definitions
- redteam-core.ts
- precommit-secretlint.sh
- handlers.rs
- api_keys.rs
- code:py (import trustloopguard as trustloop)
- insert_existing_workspace_member
- aggregate
- State
- PostgresHumanReviewAdapter
- code:text (POST /v1/traces/{trace_id}/review-events)
- store.rs
- event_service.rs
- .create_event
- Web UI Conventions
- MemoryLlmUsageStore
- enforcement.rs
- properties
- properties
- package.json
- SourceLabelEvidence
- LlmUsageRepo
- EnforcementProfile.ts
- backend-coverage.sh
- environment_deployments.rs
- fresh_pool
- knip.json
- Code of Conduct
- PostgresLabelPolicyAdapter
- GuardLogEvent
- RunnerAttackVector
- decision.schema.json
- sdk.tsx
- prepush-fast.sh
- lib.rs
- RedactedEntity
- parse_retry_after
- Write Your First Policy
- RunStoreError
- render-diagrams.sh
- evaluate_financial_policies
- StorageError
- tests.rs
- GuardEvent Redaction Spec
- authorize_workspace_admin
- .check
- financial_actions.rs
- budget_alerts.rs
- build_postgres_layer
- financial_actions_integration.rs
- team.rs
- FinancialExecutionResult
- .create_event
- .query
- Channel
- Security Policy
- properties
- RunnerDocumentTemplate
- TrustLoopGuard Hardening v2 — Attack-Grounded Policy Synthesis
- GatewayState
- components.json
- policy_repo.rs
- .start_run
- effective_checker_modes
- severity
- PolicyStore
- LlmClient
- tests.rs
- embedder.rs
- require_approved_user
- read_filter
- sync-recipes.ts
- code:sh (pnpm demo:job)
- key.rs
- properties
- HnswIndex
- code:text (Browser analytics UI)
- OpenRouterClient
- resolved_event
- code:sh (pnpm demo:n8n:bridge)
- TokenBudget
- load_agent_str
- monitoring_integration.rs
- validation.rs
- mod.rs
- guardrails.rs
- financial_repo.rs
- client.ts
- Crates
- redteam-runner.schema.json
- test_events.py
- main.rs
- TierResult
- docs-auth.ts
- scripts
- code:sh (pnpm --filter @trustloopguard/example-typescript start \)
- human_review.rs
- .from_response
- put_llm_price
- Common Workflows
- main.rs
- input.tsx
- UserStoreError
- str
- properties
- ParamLimit
- PolicyValidateResponse
- ParamLimit
- RedteamDispatchRequest.ts
- Client
- code:sh (pip install -e sdks/python)
- AnthropicGatewayProvider
- page.tsx
- router
- run.rs
- github.ts
- Event Engine
- code:sh (TL_SERVER_URL=http://127.0.0.1:8080 \)
- Policy YAML Reference
- LabelPolicyStoreError
- compilerOptions
- verify_candidate
- PostgresUserAdapter
- types.ts
- code:py (retry=RetryConfig(max_attempts=1, total_budget_s=0.25))
- event_summary
- policy.rs
- seal_key_material
- properties
- LlmPricingStoreError
- page.tsx
- build_app
- validation.rs
- check_pipeline.rs
- definitions
- ToolMetadataStoreError
- Human Review Analytics Spec
- WorkflowRequirement
- HumanReviewAnalyticsResponse.ts
- Decision.ts
- compilerOptions
- devDependencies
- LlmRouter
- seed-demo.ts
- compilerOptions
- knowledge.rs
- LlmUsageStoreError
- HardenCandidate.ts
- LlmPricingRepo
- lib.rs
- trustloopguard
- http.rs
- policy
- build_app_state
- request
- tests.rs
- RunnerPlanRequest
- $ref
- 4. Goal-Driven Execution
- SourceLabelPolicy
- MemoryBudgetAlertStore
- retry_integration.rs
- view_from_record
- llm_usage.rs
- check_and_maybe_regenerate
- MemoryHumanReviewStore
- mod.rs
- .analytics
- 1. Think Before Coding
- budget.rs
- Plugin contract
- RunnerReport
- Policy Cookbook
- route.ts
- MemoryToolMetadataStore
- TeamStoreError
- ConnectAgentStep.tsx
- Any
- index.mdx
- BudgetAlertStoreError
- PostgresToolMetadataAdapter
- Architecture
- Team & invites
- 2. Simplicity First
- SettingsStore
- core.ts
- compilerOptions
- parse_body
- events_integration.rs
- fresh_repo
- agents.rs
- fresh_repo
- PostgresLlmPricingAdapter
- Red-Team Dispatch
- @trustloopguard/sdk
- layout.tsx
- scripts
- SDK publishing
- code:bash (curl -X POST $TLG_URL/v1/check \)
- create_my_workspace
- api_error_response
- wire.rs
- MockRefundClient
- .submit_event
- header_value
- marketing-event-link.tsx
- JsonSchema
- MemoryPolicyStore
- Authorization
- Gateway
- RunnerPlanResponse
- feature_request.md
- route.ts
- PolicyEditorDialog.test.tsx
- code:json ({)
- AuthUserState
- ApiError
- HumanReviewStoreError
- runs_integration.rs
- ProvenanceMap
- fresh_repo
- LlmUsageBucketsResponse.ts
- FinancialMandate
- LiveKitSupportAgent
- code:sh (pnpm demo:chat)
- LiveKit agent guardrail demo
- Agent-hardening loop
- Red-Team Report Sharing
- RunnerHandle
- Red-Team Runner Contract v1
- Integration & Interception — How TrustLoopGuard Hooks an Agent
- compilerOptions
- guard-modes.mdx
- properties
- properties
- policy_ast.rs
- LabelBasisSet
- properties
- 3. Surgical Changes
- .generate_guardrails
- api_error_response
- code:sh (pnpm demo:chat:interactive)
- HandlerCtx
- properties
- definitions
- source-label-policy.schema.json
- params
- properties
- Analytics Dashboards
- Embedder
- devDependencies
- code:text (Customer / integrator runtime)
- run.sh
- code:text (1. [Step] -> verify: [check])
- code:bash (make quickstart)
- RunnerAttackSession
- source_chain
- properties
- code:block2 (CheckRequest)
- proxy_anthropic_messages
- .list_policies
- docs
- package.json
- service.rs
- WorkflowDefinition
- LabelResolution
- validate_create_action
- CheckerRun
- query_parts
- Financial Authorization
- Financial Authorization Contract TDD Evidence
- MemoryKnowledgeStore
- proxy.ts
- auth.ts
- Environments
- TrustLoopGuard concepts
- Runs
- Merge gates
- budget_alert.rs
- checks.rs
- dashboard-widgets.tsx
- SignalEvidence
- Policies
- README.md
- hallucination.md
- semantic_policy.md
- any_policy_summary
- insert_trace
- The three rules
- route.ts
- llm_usage.rs
- Web Dashboard And Authentication
- tool-metadata.schema.json
- seo.ts
- authority.md
- tone.md
- page.tsx
- Human Review Analytics
- generate-openapi-docs.mjs
- EntityVersionListResponse.ts
- WorkspaceEnvironmentListResponse.ts
- layout.tsx
- OpenAiClient
- next.config.mjs
- auth.rs
- default_settings
- KnowledgeStoreError
- AnalyticsFact
- source.config.ts
- generate_guardrails
- next.config.ts
- postcss.config.mjs
- next.config.ts
- postcss.config.mjs
- Glossary
- README.md
- AiEditRequest.ts
- AiEditResponse.ts
- AuthRequest.ts
- AuthResponse.ts
- ChangePasswordRequest.ts
- CreateWorkspaceEnvironmentRequest.ts
- CreateWorkspaceRequest.ts
- EntityVersionDetail.ts
- OAuthIdentityRequest.ts
- UpdateWorkspaceEnvironmentRequest.ts
- code:text (agent drafts output)
- code:text (SDK / agent runtime)
- code:sh (pnpm install)
- code:sh (make quickstart)
- code:bash (export TL_API_KEY=dev-admin)
- code:sh (cargo run -p example-rust -- "show me my password" "here it )
- code:bash (pnpm --filter web dev)
- code:ts (import OpenAI from 'openai';)
- code:sh (pnpm demo:chat)
- code:sh (cargo run -p tl-cli -- policy validate policies/refund-guara)
- code:ts (import OpenAI from 'openai';)
- code:text (POST /v1/gateway/{route_id}/openai/chat/completions)
- code:ts (import Anthropic from '@anthropic-ai/sdk';)
- code:text (POST /v1/gateway/{route_id}/anthropic/v1/messages)
- Dashboard setup
- code:ts (import { GuardMode, guard } from '@trustloopguard/sdk';)
- code:ts (const guardrail = guard({)
- code:python (async def regenerate_reply(feedback: trustloop.RegenerateFee)
- index.mdx
- proxy_provider_request
- RecordingTraceStore
- properties
- proxy_healthcare_agent.py
- validate_create_event
- fresh_pool
- prepare_streaming_request
- code:sh (cargo run -p tl-cli -- policy validate policies/example.yaml)
- next-env.d.ts
- code:sh (pnpm install)
- code:sh (DOCS_PASSWORD=replace-with-a-secret)
- STEPS
- REASONS
- next-env.d.ts
- { POST }
- providers
- DocsPageProps
- ButtonProps
- onSelectedRowKeysChange
- user
- labelVariants
- SheetContentProps
- sheetVariants
- apiKeyColumns
- runColumns
- policyColumns
- payloadWithoutNote
- req
- fetchMock
- headers
- drizzle-orm
- postgres
- db:push
- client
- db
- GuardrailGenerateResponse
- GuardrailListResponse
- Policy
- PolicyDocument
- Response
- T
- AnalyticsCatalogMetric
- AnalyticsChartType
- AnalyticsDashboardView
- AnalyticsDashboardViewConfig
- AnalyticsDashboardViewListResponse
- AnalyticsDashboardWidget
- AnalyticsDimension
- AnalyticsFacet
- AnalyticsFacetCatalogResponse
- AnalyticsFilter
- AnalyticsMetric
- AnalyticsQueryPoint
- AnalyticsQueryRequest
- AnalyticsQueryResponse
- AnalyticsWidgetLayout
- ApiKeyBatchRevokeRequest
- ApiKeyBatchRevokeResponse
- ApiKeyListResponse
- Channel
- CreateAnalyticsDashboardViewRequest
- CreateApiKeyRequest
- CreateApiKeyResponse
- CreateHumanReviewEventRequest
- CreateKnowledgeSourceRequest
- CreateWorkspaceEnvironmentRequest
- DashboardApiKey
- DashboardKnowledgeSourceKind
- DataHandlingMode
- HumanReviewAnalyticsResponse
- HumanReviewAnalyticsSummary
- HumanReviewEvent
- HumanReviewEventListResponse
- HumanReviewGroupRow
- HumanReviewOutcomeCounts
- HumanReviewPolicyRow
- HumanReviewReasonRow
- HumanReviewWorkflowStepRow
- KnowledgeFileInput
- KnowledgeFileMetadata
- KnowledgeSourceDocument
- KnowledgeSourceFileResponse
- KnowledgeSourceListResponse
- KnowledgeSourceStatus
- RedactedEntity
- RedactionInfo
- RedactionMode
- RedactionStatus
- Formatter
- HumanReviewOutcome
- Value
- RunEventKind
- RunKind
- RunListResponse
- RunStatus
- Severity
- TlError
- TraceSummary
- TriggeredPolicy
- UpdateAnalyticsDashboardViewRequest
- UpdateWorkspaceEnvironmentRequest
- WorkspaceEnvironment
- WorkspaceEnvironmentListResponse
- WorkspaceSettings
- InviteLookupResponse
- MockRunner
- Arc
- CancellationToken
- HandlerCtx
- Policy
- TierOutput
- Vec
- MockClient
- Duration
- gateway.mdx
- CreateRunEventRequest
- CreateRunRequest
- Duration
- GuardrailGenerateResponse
- GuardrailListResponse
- HeaderMap
- RunDetail
- RunEventListResponse
- RunEventSummary
- T
- TraceListResponse
- UpdateRunRequest
- MemoryAgentStore
- AgentProfile
- ApiErrorCode
- HashMap
- Result
- RwLock
- Self
- StatusCode
- Vec
- AnalyticsUserId
- ApiErrorCode
- HashMap
- RwLock
- StatusCode
- Uuid
- ApiErrorCode
- StatusCode
- MemoryUserStore
- PasswordError
- ApiErrorCode
- HashMap
- Json
- Response
- RwLock
- Self
- State
- StatusCode
- MemoryApiKeyRecord
- MemoryApiKeyStore
- MemorySettingsStore
- ApiErrorCode
- Extension
- HeaderMap
- InternalServiceContext
- Json
- Response
- RwLock
- State
- StatusCode
- UserContext
- MemoryEnvironmentStore
- HashMap
- RwLock
- Self
- Vec
- WorkspaceEnvironment
- MemoryHumanReviewStore
- HashMap
- HeaderMap
- Item
- Iterator
- Json
- Path
- RwLock
- State
- Uri
- MemoryKnowledgeStore
- ApiErrorCode
- CreateKnowledgeSourceRequest
- HashMap
- HeaderMap
- Json
- KnowledgeSourceDocument
- KnowledgeSourceFileResponse
- Path
- Response
- Result
- RwLock
- Self
- State
- StatusCode
- LlmModelPrice.ts
- Vec
- ApiDoc
- AppState
- AuthConfig
- Instant
- Json
- LlmClient
- Next
- Request
- Response
- RunStoreError
- State
- StatusCode
- GuardrailState
- MemoryPolicyRecord
- MemoryPolicyStore
- ParsedPolicyBody
- Action
- AgentStore
- ApiErrorCode
- Bytes
- EntityVersionDetail
- EntityVersionListResponse
- HashMap
- HeaderMap
- JsonSchema
- Path
- Policy
- PolicyDocument
- PolicySummary
- PolicyValidationIssue
- Result
- RwLock
- Self
- State
- StatusCode
- Vec
- MemoryRunStore
- HashMap
- Item
- Iterator
- Json
- Path
- Response
- RunEventKind
- RwLock
- Self
- State
- TraceSummary
- Uri
- Vec
- InviteLookupRecord
- MemoryTeamState
- MemoryTeamStore
- ApiErrorCode
- Extension
- HeaderMap
- Json
- Path
- Response
- Result
- RwLock
- State
- StatusCode
- StorageError
- TeamRepo
- UserContext
- Uuid
- TeamRepoAdapter
- AnalyticsFact
- MetricAccumulator
- NewViewRecord
- AnalyticsDashboardView
- AnalyticsDashboardViewConfig
- AnalyticsWidgetLayout
- CreateAnalyticsDashboardViewRequest
- DateTime
- Item
- Iterator
- Option
- String
- UpdateAnalyticsDashboardViewRequest
- Utc
- Value
- Vec
- ViewRecord
- ApiKeyAuthRecord
- ApiKeyRecord
- NewApiKeyRecord
- DashboardApiKey
- Uuid
- Vec
- DateTime
- FailMode
- GatewayInputAction
- GatewayOutputAction
- GatewayProviderKind
- NewEnforcementProfile
- NewGatewayProviderConnection
- NewGatewayRoute
- ResponseMode
- RetentionMode
- Utc
- Vec
- GroupAccumulator
- PolicyAccumulator
- F
- HumanReviewAnalyticsFilter
- HumanReviewAnalyticsResponse
- HumanReviewOutcome
- T
- Value
- WorkflowAccumulator
- Vec
- ContainerAsync
- PostgresImage
- CreateRunEventRequest
- DateTime
- HashMap
- HumanReviewOutcome
- RunEventKind
- RunEventSummary
- TraceReviewLookupRow
- TraceSummary
- Utc
- Uuid
- Value
- RunStats
- InviteLookup
- InviteRow
- MemberRow
- DateTime
- MyWorkspace
- Option
- String
- Utc
- Uuid
- Vec
- fresh_repos
- WorkspaceRole
- UserNameRow
- str
- str
- code:ts (import { GuardMode, guard } from '@trustloopguard/sdk';)
- code:sh (pip install -e sdks/python)
- code:sh (TL_API_KEY=dev-admin \)
- code:json ({)
- Gateway proxy
- code:yaml (agent_id: my-bot)
- code:yaml (agent_id: string                 # required, non-empty)
- code:yaml (knowledge_sources:)
- code:yaml (tone:)
- code:yaml (# policies/no-guarantees.yaml)
- code:yaml (authority:)
- code:bash (# Register (creates or replaces by agent_id))
- code:text (POST /v1/identity/oauth-session)
- Workspace API keys (future)
- code:yaml (id: refund-promise)
- code:bash (cargo run -p tl-codegen           # write)
- `tl-storage` — decision log
- code:text (Customer app -> /v1/gateway/... -> input check -> provider -)
- code:text (baseURL = https://<server>/v1/gateway/<route_id>/openai)
- code:text (baseURL = https://<server>/v1/gateway/<route_id>/anthropic)
- code:block2 (fn check(draft: Draft, ctx: Context) -> Decision)
- code:block3 (Draft {)
- code:block4 (Context {)
- code:block5 (Decision {)
- code:block6 (fn push(chunk: String) -> StreamDecision)
- code:bash (pnpm docs:diagrams)
- code:text (Workspace -> Agent -> Run -> Run event -> Trace / Decision)
- code:text (POST   /v1/runs)
- code:bash (git tag -l 'sdk-v0.0.6')
- code:bash (git fetch origin main --tags)
- code:bash (git rev-parse sdk-v0.0.6)
- code:bash (gh run list --workflow "Publish SDK" --limit 5)
- code:bash (git tag -f sdk-v0.0.6 <commit>)
- code:text (+----------+         +----------+         +----------+)
- code:text (+---------------+       POST /v1/team/invites       +-------)
- Existing users
- Public lookup
- Why signup-with-token, not full JWT
- code:block2 (t=0       all three tiers start in parallel)
- code:rust (let cancel = CancellationToken::new();)
- code:block4 (1. Built-in defaults (we ship))
- code:block5 (Layer 4: Framework adapters     (Vercel AI SDK, LangChain, O)
- code:typescript (const reply = await tl.guard({)
- Data model
- Goals
- Non-goals
- Shared resources and generated contracts
- Web dashboard and authentication spec
- code:ts (interface DataTableColumn<T> {)
- code:bash (brew install d2)
- code:text (customer app -> provider SDK -> provider)
- code:text (customer app -> crates/tl-server gateway endpoint -> provide)
- code:text (browser -> apps/web same-origin API route -> crates/tl-serve)
- code:text (OpenAI-compatible:)
- code:text (X-TrustLoopGuard-Verdict: blocked | escalated)
- code:text (apps/web/app/api/gateway/*)
- code:text (crates/tl-server/src/gateway.rs)
- code:text (customer app -> OpenAI/Anthropic SDK with TrustLoopGuard bas)
- code:ts (import OpenAI from 'openai';)
- code:text (POST /v1/gateway/{route_id}/openai/chat/completions)
- code:ts (import Anthropic from '@anthropic-ai/sdk';)
- code:text (POST /v1/gateway/{route_id}/anthropic/v1/messages)
- code:text (Provider connection)
- code:text (proxy_openai_chat_completions)
- code:text (TrustLoopGuard endpoint:)
- code:block1 (one-time, off the hot path)
- code:bash (pip install trustloopguard)
- code:python (import os)
- code:bash (curl -X POST $TLG_URL/v1/agents \)
- code:ts (onError: (_err, _draft) => "I'm having trouble right now — l)
- code:python (on_error=lambda _err, _draft: "I'm having trouble right now )
- code:yaml (# policies/agents/acme-support-v3.yaml)
- code:bash (curl -X POST https://your-trustloopguard/v1/agents \)
- code:python (import trustloopguard as trustloop)
- code:ts (import { GuardMode, guard } from "@trustloopguard/sdk";)
- code:python (async def regenerate_reply(feedback: trustloop.RegenerateFee)
- code:ts (const guardrail = guard({)
- code:bash (pnpm add @trustloopguard/sdk)
- code:ts (import { Client, guard } from "@trustloopguard/sdk";)
- code:bash (curl -X POST \)
- code:bash (curl -H "Authorization: Bearer $TL_API_KEY" \)
- code:bash (curl -X DELETE \)
- code:yaml (match:)
- code:yaml (when:)
- code:bash (cargo run -p tl-cli -- policy-lint docs/policies/examples/re)
- code:text (POST /v1/policies/validate)
- code:yaml (agent_id: baker-9000)
- code:bash (# Register the agent first.)
- code:text (universal built-ins)
- code:bash (cargo run -p tl-cli -- policy validate policies/refund-promi)
- code:yaml (id: refund-guarantee)
- code:bash (cargo run -p tl-cli -- policy validate policies/refund-guara)
- code:text (ok: policy `refund-guarantee` valid)
- code:text (local file -> tl-policy parser -> Policy)
- code:bash (cargo run -p tl-cli -- policy push policies/refund-guarantee)
- code:bash (cargo run -p tl-cli -- policy pull refund-guarantee \)
- code:yaml (id: refund-guarantee)
- code:text (match.regex: regex failed to compile)
- code:text (rewrite: rewrite is required when action is rewrite)
- code:yaml (action: rewrite)
- code:text (match.literal: literal matcher must not be empty)
- code:text (when.agents[0]: must not be empty)
- code:yaml (action: rewrite)
- code:yaml (id: Refund Guarantee)
- code:yaml (description: Prevents agents from guaranteeing refunds.)
- code:yaml (when:)
- code:yaml (match:)
- code:ts (const run = await client.startRun({)
- code:text (Customer app raw data)
- code:text (POST /v1/check raw or sanitized request)
- code:text (CheckRequest)
- code:text ([PERSON_NAME_1] has SIN [SIN_1] and income [INCOME_AMOUNT_1])
- code:text ([customer_info] has private data.)
- code:rust (pub struct CheckRequest {)
- code:rust (pub struct Decision {)
- code:json ({)
- code:json ({)
- code:sh (# Terminal 1)
- code:sh (./loadtest/run.sh allow -n 10000 -c 200)
- code:bash (pnpm recipes:update)
- code:bash (pnpm recipes:check)
- code:md (<!-- BEGIN recipe:output-boundary-guard:typescript -->)
- next
- float
- str
- bool
- float
- int
- str
- InviteLookupResponse
- bool
- float
- int
- Run a check and dispatch the appropriate callback. Returns the     string the ca
- float
- int
- str
- float
- code:ts (import { guard, GuardMode } from '@trustloopguard/sdk';)
- code:ts (import { guard } from '@trustloopguard/sdk';)
- code:ts (import { Client } from '@trustloopguard/sdk';)
- code:ts (import OpenAI from 'openai';)
- code:ts (import Anthropic from '@anthropic-ai/sdk';)
- client
- fetchSpy
- body
- client
- { client, fetchSpy }
- events
- fetchSpy
- guardrail
- init
- onAllow
- onBlock
- onError
- onEscalate
- onRevise
- regenerate
- seen
- client
- fetchSpy
- headers
- client
- fetchSpy
- c
- d
- client
- fetchSpy
- setup.ts
- TierRunner
- code:text (Customer / integrator runtime)
- code:text (1. [Step] -> verify: [check])
- code:text (source of truth)
- code:bash (# Clone and enter the repo)
- code:block2 (<type>: <short description>)
- 0a. Set up secrets (one-time, per machine)
- 0b. Start the server (all languages need this)
- 1. Rust
- 2. Python
- 3. TypeScript
- code:bash (brew install dopplerhq/cli/doppler   # or see docs.doppler.c)
- code:bash (pnpm demo:chat               # scripted live-chat scenarios)
- code:bash (pnpm test:backend)
- code:bash (pnpm coverage:backend)
- code:bash (cargo install cargo-llvm-cov)
- code:bash (make backend-test-db)
- code:bash (make server                          # = doppler run -- carg)
- code:bash (TL_LOG_FORMAT=pretty doppler run -- cargo run -p tl-server)
- code:bash (cargo run -p example-rust -- "show me my password" "here it )
- code:bash (pip install -e sdks/python)
- code:bash (pnpm install)
- code:block7 (verdict       : block)
- Repo philosophy
- TrustLoopGuard
- What is TrustLoopGuard?
- rules
- openapi.rs
- defaults.rs
- fresh_pool
- Red-team harden (policy synthesis)
- UpsertLlmModelPriceRequest.ts
- index.mdx
- HumanReviewAnalyticsFilter
- setup.ts
- Agent Breakaway Arena
- llm_pricing.rs
- Verdict
- submit_event
- index.mdx
- .__init__
- hash_password
- api_error
- gateway_routes
- BudgetAlertFiring
- latency_ms
- LlmPricingStore

## God Nodes (most connected - your core abstractions)
1. `StorageError` - 360 edges
2. `cn()` - 182 edges
3. `Client` - 138 edges
4. `State` - 124 edges
5. `FinancialStoreError` - 112 edges
6. `AsyncClient` - 106 edges
7. `AppState` - 83 edges
8. `Policy` - 81 edges
9. `Client` - 75 edges
10. `proxyRustJson()` - 72 edges

## Surprising Connections (you probably didn't know these)
- `createOutputGuard()` --indirect_call--> `decision()`  [INFERRED]
  sdks/typescript/src/guard.ts → apps/mcp-server/src/handlers.test.ts
- `DecisionHandler` --indirect_call--> `decision()`  [INFERRED]
  sdks/typescript/src/guard.ts → apps/mcp-server/src/handlers.test.ts
- `main()` --indirect_call--> `event()`  [INFERRED]
  demo/dispute/scenarios.ts → apps/mcp-server/src/handlers.test.ts
- `entrypoint()` --calls--> `RetryConfig`  [INFERRED]
  demo/livekit/guarded_healthcare_agent.py → sdks/python/src/trustloopguard/retry.py
- `AttacksPanel()` --indirect_call--> `summary()`  [INFERRED]
  apps/web/app/attacks/_components/attacks-panel.tsx → apps/web/app/r/[token]/report-document.test.ts

## Import Cycles
- 2-file cycle: `crates/tl-server/src/redteam/mod.rs -> crates/tl-server/src/redteam/share.rs -> crates/tl-server/src/redteam/mod.rs`
- 2-file cycle: `crates/tl-server/src/policies.rs -> crates/tl-server/src/policies/authoring.rs -> crates/tl-server/src/policies.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_job_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/trace_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_plan_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/user_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/knowledge_repo.rs -> crates/tl-storage/src/lib.rs -> crates/tl-storage/src/knowledge_repo.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_report_share_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/gateway_repo.rs -> crates/tl-storage/src/lib.rs -> crates/tl-storage/src/gateway_repo.rs`

## Communities (1833 total, 648 thin omitted)

### Community 1 - "Client"
Cohesion: 0.06
Nodes (63): Client, Synchronous TrustLoopGuard client.      Args:         base_url: TrustLoopGuard s, guard(), Create a simple async guard or run the legacy sync guard.      New integrations, action_body(), approval_body(), decision_receipt_body(), financial_policy_body() (+55 more)

### Community 2 - "GuardEvent"
Cohesion: 0.12
Nodes (19): Action, EventKind, GuardEvent, Principal, Action, CheckerRun, EventKind, Option (+11 more)

### Community 3 - "cn"
Cohesion: 0.04
Nodes (90): AgentFilter(), AppSidebar(), AppSidebarProps, data, NavGroup, NavItem, NavMain(), NavSecondary() (+82 more)

### Community 6 - "Enum"
Cohesion: 0.07
Nodes (87): AsyncFinancialOperation, _AsyncRunContext, _AsyncRunEventContext, FinancialOperation, CreateRunEventRequest, RunEventSummary, HTTP client for TrustLoopGuard's GuardEvent runtime contract.  Sync and async va, Async variant of ``Client.create_run_event``. (+79 more)

### Community 9 - "oauth.rs"
Cohesion: 0.12
Nodes (38): AuthCode, authorization_server_metadata(), authorize(), AuthorizeRequest, client_redirect_uris(), code_is_single_use(), issue_tokens(), issuer() (+30 more)

### Community 10 - "FinancialActionsContent.tsx"
Cohesion: 0.04
Nodes (74): Badge(), badgeVariants, DataTable(), DataTableColumn, columns, Row, rows, EmptyState() (+66 more)

### Community 11 - "dashboard-data.ts"
Cohesion: 0.02
Nodes (165): ChangePasswordCard(), AccountPage(), AgentsPage(), AnalyticsPage(), AnalyticsSearchParams, ApiKeysPage(), escapeHeaderValue(), GET() (+157 more)

### Community 12 - "GatewayPageContent.tsx"
Cohesion: 0.02
Nodes (121): compareLabel(), formatDate(), ReportShareCard(), TTL_OPTIONS, relativeTime(), VersionPicker(), VersionPickerProps, DialogContent (+113 more)

### Community 13 - "Integrating TrustLoopGuard"
Cohesion: 0.12
Nodes (16): Async, Bear-trap checklist, Fail-open vs fail-closed, Financial actions and receipts, Guard modes, Integrating TrustLoopGuard, LLM/model route failures, MCP server (+8 more)

### Community 18 - "proxyRustJson"
Cohesion: 0.04
Nodes (54): POST(), POST(), RouteContext, proxyMock, GET(), RouteContext, proxyMock, RouteContext (+46 more)

### Community 19 - "Field-by-field"
Cohesion: 0.10
Nodes (21): 1. Putting banned vocabulary in `tone.forbidden`, 2. Listing categories instead of commitments in `authority.cannot_promise`, `agent_id`, Agent profile — field reference, `authority.can_promise`, `authority.cannot_promise`, `display_name`, `escalation_triggers` (+13 more)

### Community 22 - "PolicyEditorDialog.tsx"
Cohesion: 0.04
Nodes (71): POST(), requestSchema, withOwnerAgent(), ACTION_LABEL, actionVariant(), joinList(), MATCH_TYPE_LABEL, matchSummary() (+63 more)

### Community 24 - "redteam.rs"
Cohesion: 0.13
Nodes (41): AttackVector, ComparedAttackStatus, CreateReportRequest, empty_json_object(), HardenCandidate, HardenCandidateOperation, HardenRejection, HardenRejectionReason (+33 more)

### Community 25 - "settings_update.rs"
Cohesion: 0.15
Nodes (27): app_with_owner(), environment_checker_modes_get_without_override_returns_all_inherit(), environment_checker_modes_round_trip(), get_request(), patch_settings_is_scoped_by_workspace_header(), patch_settings_rejects_invalid_mode_string(), patch_settings_rejects_non_numeric_retention_days(), patch_settings_rejects_unknown_default_action() (+19 more)

### Community 26 - "types.py"
Cohesion: 0.02
Nodes (190): BaseModel, AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone, AllowedSource, AnalyticsCatalogDimension (+182 more)

### Community 27 - "tests.rs"
Cohesion: 0.06
Nodes (50): new_trace_id(), HumanReviewOutcome, Option, String, Value, Vec, TraceListResponse, TraceSummary (+42 more)

### Community 28 - "Client"
Cohesion: 0.08
Nodes (8): Client, RunContextStore, stringifyJson(), GuardEvent, GuardrailGenerateResponse, PolicyDocument, RunSummary, TraceListResponse

### Community 29 - "errors.ts"
Cohesion: 0.04
Nodes (51): ClientOptions, CODE_TO_CLASS, codeFromHttpStatus(), Decode, DEFAULT_RETRIABLE, Forbidden, fromResponse(), Gone (+43 more)

### Community 36 - "TrustLoopGuard demos"
Cohesion: 0.25
Nodes (7): Agentic refund authorization, Bring your own agent, LiveKit, Money agent — guarded scenarios (flagship), NorthPay dispute, Stripe refund agent, TrustLoopGuard demos

### Community 37 - "Result"
Cohesion: 0.08
Nodes (58): action_from_record(), approval_from_record(), clean_operation(), clean_optional(), clean_required(), enum_from_text(), enum_text(), event_from_record() (+50 more)

### Community 38 - "AnalyticsChartGrid.tsx"
Cohesion: 0.05
Nodes (59): AnalyticsChartGrid(), AnalyticsChartGridProps, AnalyticsWidget(), applyGridOrder(), DEFAULT_LAYOUT, DEFAULT_VIEW, DIMENSION_LABELS, dimensionLabel() (+51 more)

### Community 39 - "param_auth.rs"
Cohesion: 0.09
Nodes (44): origin_str(), Origin, source(), allowed(), authority_param(), content_bearing_params_are_ignored(), content_param(), correct_source_yields_no_findings() (+36 more)

### Community 40 - "PostgresGatewayAdapter"
Cohesion: 0.13
Nodes (16): gateway_store_error(), PostgresGatewayAdapter, Arc, EnforcementProfile, EnforcementProfilePatch, GatewayProviderConnection, GatewayRoute, GatewayRoutePatch (+8 more)

### Community 42 - "llm_pricing.rs"
Cohesion: 0.15
Nodes (20): cost_minor(), default_table(), deployment_prefixes_suffix_match(), known_model_prices_exactly(), LlmPricingTable, ModelPrice, negative_tokens_clamp_to_zero(), normalize_model() (+12 more)

### Community 43 - "latest_review_outcomes"
Cohesion: 0.15
Nodes (20): latest_review_outcomes(), parse_review_outcome(), DateTime, DbConnection, DbPool, Debug, Formatter, HashMap (+12 more)

### Community 44 - "SDK-Driven Development at TrustLoopGuard"
Cohesion: 0.15
Nodes (13): Direct event submission, How features are built (the loop), MCP adapter, Out of scope, Publishing, Required CI gates, Reviewer checklist, Run grouping helper (+5 more)

### Community 47 - "tests.rs"
Cohesion: 0.06
Nodes (75): resolve_environment_id(), HeaderMap, Response, Result, String, workspace_id_from_headers(), account_workflow_profile(), create_report_mints_share_for_complete_job() (+67 more)

### Community 48 - "create_knowledge_source"
Cohesion: 0.17
Nodes (17): create_knowledge_source(), get_knowledge_source_file(), list_knowledge_sources(), CreateKnowledgeSourceRequest, HeaderMap, Json, Path, Response (+9 more)

### Community 50 - "label.rs"
Cohesion: 0.19
Nodes (17): Confidentiality, Integrity, LabelBasis, LabelBasisSet, LabelPolicyStatus, LabelResolution, Labels, Origin (+9 more)

### Community 52 - "auth.rs"
Cohesion: 0.10
Nodes (58): blank_principal_is_stored_as_unbound(), create_api_key_request_with_principal(), internal_bearer_can_revoke_workspace_keys(), internal_bearer_with_forwarded_user_can_issue_workspace_key_used_by_sdk_runtime(), internal_bearer_without_forwarded_user_cannot_issue_workspace_key(), key_bound_to_principal_carries_it_into_handler_context(), key_without_principal_keeps_context_principal_none(), local_dev_missing_forwarded_user_is_unauthorized() (+50 more)

### Community 53 - "Ownership"
Cohesion: 0.17
Nodes (11): 1. Lane ownership, 2. Critical-path crates, 3. Contracts (the only cross-lane surface), 4. Wire-format versioning, 5. Demo independence checkpoint, 6. Conflict resolution, 7. When to split further, Founder A — Engine + Plugin SDK (+3 more)

### Community 55 - "Domain terms"
Cohesion: 0.03
Nodes (61): Action vs Verdict, Agent, Agent profile, Approval rule, Attack success rate, Authority-bearing parameter, Automated intervention, Benign utility (+53 more)

### Community 56 - "RunRepo"
Cohesion: 0.14
Nodes (17): CreateRunRequest, DbConnection, DbPool, Debug, Formatter, Option, Result, RunKind (+9 more)

### Community 57 - "agent.rs"
Cohesion: 0.13
Nodes (19): AgentAuthority, AgentScope, AgentTone, AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone (+11 more)

### Community 58 - "tests.rs"
Cohesion: 0.36
Nodes (10): allow_output(), default_runner_with_no_policies_yields_allow(), different_request_misses_cache(), empty_engine_allows(), req(), second_identical_request_hits_cache(), three_allow_tiers_yield_allow_with_three_results(), tier1_block_cancels_tiers_2_and_3() (+2 more)

### Community 60 - "scenarios.core.ts"
Cohesion: 0.14
Nodes (22): executePayment(), PaymentRequest, PaymentResult, simulatedLedger, StripePaymentIntent, assertEnforced(), main(), makeDecision() (+14 more)

### Community 61 - "report.rs"
Cohesion: 0.13
Nodes (33): ComparedAttackStatus, aggregate(), aggregates_exclude_clean_control_from_denominator(), blocked_and_clean_are_informational_with_no_evidence(), build_report(), categorize(), compared_attacks(), compared_status() (+25 more)

### Community 62 - "profile_record_to_wire"
Cohesion: 0.06
Nodes (42): GatewayRepo, EnforcementProfile, EnforcementProfilePatch, NewEnforcementProfile, Result, Vec, parse_fail_mode(), parse_input_action() (+34 more)

### Community 63 - "AsyncClient"
Cohesion: 0.05
Nodes (38): RunListResponse, AsyncClient, CreateFinancialActionRequest, FinancialActionDecisionReceipt, FinancialActionListResponse, FinancialActionRecord, FinancialApprovalRequestListResponse, FinancialMandateListResponse (+30 more)

### Community 64 - "Gateway Proxy Runtime Branch Guide"
Cohesion: 0.15
Nodes (12): Configuration Objects, Current Limits, Dashboard Proxy vs Runtime Proxy, Files to Read in Order, Gateway Proxy Runtime Branch Guide, How a Customer Routes Through the Proxy, One-Sentence Model, Provider Forwarding (+4 more)

### Community 65 - "EnvironmentStoreError"
Cohesion: 0.07
Nodes (48): create_environment(), delete_environment(), environment_id_from_headers(), EnvironmentState, EnvironmentStoreError, list_environments(), ensure_default(), MemoryEnvironmentStore (+40 more)

### Community 66 - "properties"
Cohesion: 0.09
Nodes (22): properties, required, type, anyOf, Action, ToolMetadata, type, default (+14 more)

### Community 68 - "GuardEvent.ts"
Cohesion: 0.06
Nodes (30): GuardToolCallOptions, Action, AllowedSource, ApprovalRule, Confidentiality, EventKind, Integrity, LabelBasis (+22 more)

### Community 69 - "rustApiForAuthorizedWorkspace"
Cohesion: 0.06
Nodes (28): GET(), PUT(), RouteContext, stringListSchema, AGENT, MockRustApiError, MockWorkspaceAccessError, rustMock (+20 more)

### Community 70 - "scripts"
Cohesion: 0.07
Nodes (29): scripts, build, codegen, codegen:check, coverage:backend, coverage:backend:lcov, coverage:frontend, dead-code:check (+21 more)

### Community 71 - "guard.ts"
Cohesion: 0.09
Nodes (22): DemoMetric, Metrics, percentile(), Channel, CreateRunEventRequest, Decision, addDefined(), branchFor() (+14 more)

### Community 72 - "tlClientForRequest"
Cohesion: 0.09
Nodes (25): POST(), RouteContext, POST(), RouteContext, POST(), requestSchema, DraftingClient, DraftResponse (+17 more)

### Community 73 - "errorResponse"
Cohesion: 0.07
Nodes (32): POST(), RouteContext, GET(), RouteContext, GET(), GET(), DELETE(), PATCH() (+24 more)

### Community 76 - "PolicyState"
Cohesion: 0.16
Nodes (28): batch_set_policy_enabled(), delete_policy(), get_policy(), list_policies(), parse_policy_family(), read_policy_family(), Bytes, HeaderMap (+20 more)

### Community 78 - "models.rs"
Cohesion: 0.07
Nodes (99): ApprovalRequestRecord, BudgetAlertConfigRecord, BudgetAlertFiringRecord, EnforcementProfileRecord, EntityVersionRecord, EscalationRecord, FinancialActionEventRecord, FinancialActionOutcomeRecord (+91 more)

### Community 79 - "RunDetailLiveView.tsx"
Cohesion: 0.08
Nodes (41): buildGuardFlow(), buildRows(), DeliveryInterventionDetail(), DetailItem(), displayPolicy(), displayReason(), displayUserPrompt(), EventRow() (+33 more)

### Community 80 - "synthesis.rs"
Cohesion: 0.11
Nodes (42): action_candidate_backstop_matches_review_bypass_not_policy_questions(), Candidate, classifies_action_claim_from_reply_assertion(), classifies_configured_workflow_before_generic_action(), classifies_credential_from_reply_token(), classifies_pii_from_goal(), classifies_refund_workflow_before_generic_action(), classifies_system_prompt() (+34 more)

### Community 81 - "properties"
Cohesion: 0.10
Nodes (20): items, type, properties, required, type, items, type, ApprovalRule (+12 more)

### Community 82 - "JwtSigner"
Cohesion: 0.18
Nodes (20): access_token_carries_workspace_and_type(), Claims, JwtError, JwtSigner, rejects_garbage(), rejects_wrong_secret(), round_trip_mints_and_verifies(), Arc (+12 more)

### Community 83 - "mod.rs"
Cohesion: 0.09
Nodes (61): checker_ctx(), client_submitted_checker_evidence_never_survives(), ctx_with_metadata(), DecisionComposer, enforce_mode_applies_worst_finding_to_decision(), enforce_mode_with_no_findings_keeps_decision_byte_identical(), event_pipeline_no_op_context_has_all_collaborators(), EventPipelineCtx (+53 more)

### Community 84 - "AgentRepo"
Cohesion: 0.06
Nodes (46): AgentStoreError, MemoryAgentStore, AgentProfile, Arc, HashMap, Result, RwLock, Self (+38 more)

### Community 86 - "AuthConfig"
Cohesion: 0.15
Nodes (22): AuthConfig, EnvError, require_bearer(), require_internal_bearer(), Arc, Debug, Formatter, Into (+14 more)

### Community 88 - "RunnerError"
Cohesion: 0.07
Nodes (33): RedteamPlanner, RedteamRunnerClient, Client, Error, Into, Option, Result, RunnerDispatch (+25 more)

### Community 89 - "Load test"
Cohesion: 0.29
Nodes (6): Load test, Prerequisites, Run, Scenarios, What's NOT here, What to look for

### Community 92 - "change_password"
Cohesion: 0.18
Nodes (19): AuthRequest, ChangePasswordRequest, change_password(), login(), Json, Response, signup(), change_password_same_as_current_is_400() (+11 more)

### Community 93 - "pipeline_e2e.rs"
Cohesion: 0.15
Nodes (38): approval_enforce_does_not_demote_an_engine_block(), approval_enforce_escalates_required_tool(), approval_enforce_ignores_tools_without_approval_rules(), approval_fixture(), approval_modes(), approval_off_records_nothing_and_decision_unchanged(), approval_shadow_records_hypothetical_escalate_without_changing_decision(), event_with_no_sources_and_no_provenance_yields_empty_evidence() (+30 more)

### Community 94 - "schema.rs"
Cohesion: 0.08
Nodes (28): ensure_oauth_user_exists(), ensure_user_exists(), generate_token(), invite_row_to_wire(), DbConnection, Result, String, Uuid (+20 more)

### Community 95 - "attacks-panel.tsx"
Cohesion: 0.02
Nodes (133): AttackButton(), AttackFlow(), AttackFlowProps, AttacksPanel(), AttackTranscript(), buildDocumentTemplate(), bytesToBase64(), ConsoleState (+125 more)

### Community 96 - "RedteamState"
Cohesion: 0.13
Nodes (38): resolve_environment_id(), HeaderMap, Response, Result, String, cancel_job(), create_report(), dispatch_job() (+30 more)

### Community 97 - "gateway.rs"
Cohesion: 0.17
Nodes (18): build_app(), create_common_gateway_config(), create_workspace_key(), enable_streaming_mode(), gateway_owner_id(), json_request(), read_body(), read_text() (+10 more)

### Community 99 - "WorkspaceKeyContext"
Cohesion: 0.11
Nodes (46): AnalyticsState, AnalyticsStore, analytics_user_id(), AnalyticsUserId, authorize_analytics_workspace(), forwarded_user_id(), require_workspace_member(), Arc (+38 more)

### Community 100 - "run-detail-live.ts"
Cohesion: 0.12
Nodes (32): defaultEventLabel(), eventSnapshot(), latestUserDisplayText(), objectSchema, readTracePolicy(), runDetailSnapshot, RunDetailWire, runDetailWireSchema (+24 more)

### Community 101 - "share.rs"
Cohesion: 0.11
Nodes (30): create_then_get_round_trips(), expired_share_reads_as_not_found(), generate_share_token(), is_expired(), MemoryRedteamReportShareStore, MemShare, new_share(), NewReportShare (+22 more)

### Community 103 - "checker_enforcement.rs"
Cohesion: 0.14
Nodes (45): all_none_override_inherits_workspace_modes(), app_with_approval_mode(), app_with_modes(), app_with_override(), approval_enforce_escalates_tool_requiring_approval(), approval_enforce_ignores_tools_without_approval_rules(), approval_escalation_enqueues_existing_worker_payload(), approval_shadow_keeps_decision_unchanged() (+37 more)

### Community 104 - "CheckerFinding"
Cohesion: 0.10
Nodes (21): CheckerFinding, composer_applies_worst_finding_and_copies_evidence_fields(), composer_ignores_signals_for_verdict(), composer_keeps_decision_when_no_finding_carries_a_verdict(), composer_never_downgrades_the_seeded_verdict(), composer_upgrades_rewrite_seed_and_preserves_it_against_weaker_findings(), deterministic_block_wins_over_advisory_allow_signal(), finding_with() (+13 more)

### Community 105 - "FinancialStoreError"
Cohesion: 0.11
Nodes (23): FinancialStoreError, String, enforcing_action(), financial_policy_from_request(), financial_policy_record(), FinancialAuthorizationService, ledger_idempotency_key(), policy_action() (+15 more)

### Community 106 - "path"
Cohesion: 0.08
Nodes (36): deadline_exceeded_yields_timeout(), malformed_inner_json_yields_parse_error(), non_2xx_yields_status_error(), ok_response(), openai_sends_bearer_auth_and_json_schema_body(), openrouter_adds_http_referer(), schema(), generate_404_maps_to_not_found() (+28 more)

### Community 107 - "Policy"
Cohesion: 0.06
Nodes (52): Channel, check_request_omits_absent_session_id_on_serialize(), CheckRequest, Decision, RedactedEntity, RedactionInfo, RedactionMode, RedactionStatus (+44 more)

### Community 108 - "normalization.rs"
Cohesion: 0.09
Nodes (38): seal_provider_key(), fail_mode_storage_text(), input_action_storage_text(), normalize_enforcement_profile(), normalize_enforcement_profile_patch(), normalize_gateway_route(), normalize_gateway_route_patch(), normalize_optional_text() (+30 more)

### Community 109 - "event_ingestion.rs"
Cohesion: 0.15
Nodes (38): app(), CannedLlmClient, CannedLlmResponse, direct_event_cannot_spoof_gateway_to_skip_run_stats(), direct_event_rejects_run_event_from_another_run(), direct_event_with_run_updates_run_stats(), full_evidence_flows_to_trace(), json_request() (+30 more)

### Community 110 - "SdkError"
Cohesion: 0.06
Nodes (56): Exception, CreateFinancialPolicyRequest, FinancialActionOutcome, FinancialPolicyRecord, Async variant of ``Client.create_financial_policy``., Create or update a financial spending control., code_from_http_status(), Decode (+48 more)

### Community 111 - "AnalyticsStoreError"
Cohesion: 0.14
Nodes (21): AnalyticsStoreError, MemoryAnalyticsStore, AnalyticsDashboardView, AnalyticsFacetCatalogResponse, AnalyticsQueryRequest, AnalyticsQueryResponse, CreateAnalyticsDashboardViewRequest, HashMap (+13 more)

### Community 112 - "redteam-report.ts"
Cohesion: 0.07
Nodes (33): COLORS, COMPARISON_STATUS, ComparisonSection(), Finding(), formatDate(), outcomeStyle(), pct(), ReportDocument() (+25 more)

### Community 113 - "tests.rs"
Cohesion: 0.25
Nodes (18): authority_violation_blocks(), CannedClient, ctx_with(), empty_router_yields_skipped(), FixedResolver, hallucination_violation_blocks(), no_profile_yields_skipped(), pre_cancelled_token_short_circuits() (+10 more)

### Community 116 - "create_api_key"
Cohesion: 0.17
Nodes (29): ApiKeyBatchRevokeRequest, DashboardAdminState, batch_revoke_api_keys(), create_api_key(), generate_plaintext_key(), get_environment_checker_modes(), get_settings(), list_api_keys() (+21 more)

### Community 117 - "AppState"
Cohesion: 0.17
Nodes (30): agent_routes(), analytics_routes(), auth_identity_routes(), budget_alert_routes(), dashboard_admin_routes(), environment_routes(), financial_routes(), guardrail_routes() (+22 more)

### Community 118 - "AnalyticsDashboardWidget.ts"
Cohesion: 0.11
Nodes (18): AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView, AnalyticsDashboardViewConfig, AnalyticsDashboardViewListResponse, AnalyticsDashboardWidget, AnalyticsDimension (+10 more)

### Community 119 - "RedteamReportShareRepo"
Cohesion: 0.16
Nodes (16): NewShare, parse_uuid(), RedteamReportShareRepo, ReportShareRow, DateTime, DbConnection, DbPool, Debug (+8 more)

### Community 120 - "TraceStoreError"
Cohesion: 0.22
Nodes (11): PostgresTraceAdapter, Arc, Option, Result, Self, Sender, TraceSummary, Vec (+3 more)

### Community 121 - "Technical terms"
Cohesion: 0.06
Nodes (35): Attack plan, Attack runner, Attack vector, Cache key, Cold path, Decision log, Embedded mode, Escalation worker (+27 more)

### Community 122 - "tool-runner.ts"
Cohesion: 0.10
Nodes (28): runRefundAgent(), shouldUseOpenAI(), main(), promptFromArgsOrStdin(), formatMoney(), searchOrderTool(), AgentState, initialMessages() (+20 more)

### Community 124 - "dashboard_admin_repo.rs"
Cohesion: 0.15
Nodes (27): DashboardAdminRepo, environment_checker_modes_from_record(), EnvironmentCheckerModesRecord, EnvironmentCheckerModesWriteRecord, mode_to_db(), optional_mode_to_db(), parse_data_handling_mode(), parse_enforcement_mode() (+19 more)

### Community 125 - "PolicyStoreError"
Cohesion: 0.20
Nodes (13): any_policy_document(), policy_document(), PolicyDocument, MemoryPolicyStore, Arc, EntityVersionDetail, EntityVersionListResponse, PolicyDocument (+5 more)

### Community 126 - "financial_authorization_service.rs"
Cohesion: 0.07
Nodes (62): executable_refund_request(), financial_policy(), mandate_request(), mandate_request_with_scope(), outcome(), payment_financial_policy(), payment_request(), refund_request() (+54 more)

### Community 127 - "family_parse.rs"
Cohesion: 0.07
Nodes (67): approval_requires_at_least_one_condition(), documented_family_examples_parse(), existing_content_examples_parse_via_load_any_str(), family(), family_id_uses_content_slug_rule(), family_less_yaml_parses_as_content_identical_to_load_str(), family_policies_round_trip_through_yaml_with_family_tag(), FamilyProbe (+59 more)

### Community 129 - "BudgetAlertRepo"
Cohesion: 0.11
Nodes (29): BudgetAlertRepo, NewBudgetAlertConfigParams, NewBudgetAlertFiringParams, parse_config_id(), DateTime, DbConnection, DbPool, Debug (+21 more)

### Community 130 - "analytics.rs"
Cohesion: 0.24
Nodes (23): AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView, AnalyticsDashboardViewConfig, AnalyticsDashboardViewListResponse, AnalyticsDashboardWidget, AnalyticsDimension (+15 more)

### Community 131 - "index.ts"
Cohesion: 0.03
Nodes (50): apiKeyBatchRevokeResponseSchema, apiKeySchema, revokeApiKeys(), ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse, BudgetAlertConfig, RFC-3339 (+42 more)

### Community 133 - "HnswFuzzyChecker"
Cohesion: 0.12
Nodes (21): BuildError, dedup_when_both_tiers_match_same_policy(), empty_policies_yields_no_hits(), HnswFuzzyChecker, levenshtein_catches_typo_bypass(), levenshtein_misses_unrelated_text(), literal_policy(), Arc (+13 more)

### Community 134 - "policy_cli.rs"
Cohesion: 0.19
Nodes (21): Command, find_header_end(), policy_pull_writes_source_yaml_to_file(), policy_push_posts_yaml_to_server(), policy_push_rejects_family_yaml_with_clear_error(), policy_validate_reports_valid_family_yaml(), policy_validate_reports_valid_yaml(), read_http_request() (+13 more)

### Community 135 - "tl-client.ts"
Cohesion: 0.09
Nodes (30): GET(), MyWorkspace, MyWorkspacesResponse, POST(), userFromSession(), requestSchema, ConsentForm(), Props (+22 more)

### Community 136 - "adapter.ts"
Cohesion: 0.06
Nodes (58): ArenaAdapterChatRequest, ArenaAdapterChatResult, ArenaAdapterFinishReason, ArenaAdapterHandlers, ArenaAdapterPhase, ArenaAdapterProfile, ArenaAdapterServer, ArenaAdapterVerdict (+50 more)

### Community 138 - "compilerOptions"
Cohesion: 0.12
Nodes (15): compilerOptions, esModuleInterop, exactOptionalPropertyTypes, isolatedModules, module, moduleResolution, noEmit, noUnusedLocals (+7 more)

### Community 141 - "README.md"
Cohesion: 0.09
Nodes (17): Copyable Policy Examples, Legal Advice Escalation, PII Block, Refund Guarantee Rewrite, Voice-Only Disclosure, CLI Workflow, Cloud Mode, Hybrid Mode (+9 more)

### Community 143 - "PostgresRedteamJobAdapter"
Cohesion: 0.14
Nodes (15): clamp_limit(), job_store_error(), PostgresRedteamJobAdapter, Arc, JobCounts, JobStatus, Option, RedteamAttackRecord (+7 more)

### Community 144 - "FinancialActionRecord"
Cohesion: 0.09
Nodes (27): action(), apiAction(), buildRefundRequest(), createRefundMandate(), FinancialDemoClient, REFUND_SCENARIOS, RefundScenario, runRefundDemo() (+19 more)

### Community 145 - "event_policy.rs"
Cohesion: 0.08
Nodes (59): all_literal_miss_does_not_call_semantic_judge(), any_literal_match_does_not_call_semantic_judge(), channel_name(), ClauseDecision, eval_ctx(), evaluate_event_policies(), evaluate_semantic_policy(), event_summary() (+51 more)

### Community 146 - "server.ts"
Cohesion: 0.03
Nodes (63): ClientEnv, createTrustLoopClient(), readClientOptions(), agentProfile(), createToolHandlers(), errorToolResult(), JsonObject, JsonPrimitive (+55 more)

### Community 147 - "labels.rs"
Cohesion: 0.14
Nodes (27): combine_all_trusted_is_trusted(), combine_any_untrusted_is_untrusted(), combine_confidentiality_takes_max_rank(), combine_integrity_takes_min_rank(), combine_labels(), combine_unknown_conf_outranks_public_only(), combine_unknown_without_untrusted_is_unknown(), confidentiality_rank() (+19 more)

### Community 149 - "api_error_response"
Cohesion: 0.23
Nodes (20): delete_label_policy(), get_label_policy(), invalid_origin_response(), LabelPolicyState, list_label_policies(), parse_origin(), Arc, HeaderMap (+12 more)

### Community 151 - "package.json"
Cohesion: 0.22
Nodes (8): description, engines, node, license, name, packageManager, private, version

### Community 152 - "PoliciesPageContent.tsx"
Cohesion: 0.04
Nodes (75): AgentFilterProps, MonacoDiffEditor, PolicyYamlDiffEditor(), Props, relativeTime(), VersionEntry, AlertDialog(), AlertDialogAction() (+67 more)

### Community 153 - "RedteamJobStoreError"
Cohesion: 0.11
Nodes (28): event_text(), MemoryRedteamJobStore, HashMap, JobCounts, JobStatus, Option, RedteamAttackRecord, RedteamAttackRecordFilter (+20 more)

### Community 154 - "gateway_budget.rs"
Cohesion: 0.22
Nodes (42): actions_meter_policy_does_not_gate_llm_calls(), admin_request(), at_cap_denies_without_calling_upstream(), build_app(), chat_request(), create_common_gateway_config(), create_extra_runtime_key(), create_llm_budget() (+34 more)

### Community 155 - "FinancialActionDecisionReceipt.ts"
Cohesion: 0.05
Nodes (37): TriggeredPolicy, Vec, Tier, TierResult, TierStatus, FinancialOperationSpec, ApprovalRequirement, CounterpartyRef (+29 more)

### Community 156 - "seo-landing-page.tsx"
Cohesion: 0.12
Nodes (21): metadata, Page, metadata, Page, metadata, Page, metadata, Page (+13 more)

### Community 158 - "properties"
Cohesion: 0.12
Nodes (16): $ref, default, type, type, $ref, description, type, properties (+8 more)

### Community 159 - "Result"
Cohesion: 0.12
Nodes (19): Client, Client, CreateRunEventRequest, CreateRunRequest, Decision, F, GuardEvent, Option (+11 more)

### Community 160 - "MemoryFinancialStore"
Cohesion: 0.09
Nodes (34): FinancialLedgerEntryKind, clean_optional(), clean_required(), key(), mandate_key(), MemoryFinancialStore, MemoryLedgerEntry, ApprovalRequirement (+26 more)

### Community 161 - "properties"
Cohesion: 0.10
Nodes (20): type, $ref, type, properties, agent_id, authority, display_name, scope (+12 more)

### Community 162 - "type"
Cohesion: 0.15
Nodes (14): properties, default, items, type, default, items, type, default (+6 more)

### Community 163 - "LabelPolicyProvider"
Cohesion: 0.12
Nodes (18): LabelPolicyProvider, LabelPolicyUnavailable, NoOpLabelPolicyProvider, PolicyLabelResolver, ProvenancePropagator, Arc, GuardEvent, Result (+10 more)

### Community 164 - "Repository Agent Instructions"
Cohesion: 0.15
Nodes (12): Architecture: Rust Backend Is the Source of Truth, Coding Conventions, Docs Are the Single Source of Truth (`docs/concept`), General Coding Discipline, Goal-Driven Execution, Implementation Checklist, Page Integration Expectations, Repository Agent Instructions (+4 more)

### Community 165 - "latest_review_outcomes"
Cohesion: 0.13
Nodes (15): latest_review_outcomes(), DateTime, DbConnection, HashMap, HumanReviewOutcome, Result, TraceReviewLookupRow, Utc (+7 more)

### Community 166 - "ToolMetadataRepo"
Cohesion: 0.16
Nodes (19): cache_key(), deserialize_spec(), Arc, Cache, DbConnection, DbPool, Debug, Duration (+11 more)

### Community 167 - "TierResult"
Cohesion: 0.15
Nodes (13): TierResult, format, minimum, type, elapsed_ms, status, tier, $ref (+5 more)

### Community 168 - "tier_results"
Cohesion: 0.15
Nodes (13): $ref, reasons, tier_results, triggered_policies, default, items, type, default (+5 more)

### Community 170 - "v0 Design Decisions"
Cohesion: 0.07
Nodes (30): 10. Crate alignment, 11. Build order (v0), 12. Open questions (need answers before phase 1), 13. Things deliberately not in v0, 14. Confirmation checklist, 15. Event-centered runtime (locked), 16. Enforcement is an opt-in rollout (locked), 17. Labeling strategy: structure-first, fail-closed for authority (locked) (+22 more)

### Community 171 - "Runtime Refactor Jobs"
Cohesion: 0.07
Nodes (28): Continuation Readability Pass, Current Status, Final Acceptance Gates, Phase 0: Baseline Evidence, Phase 1: Server Shell Cleanup, Phase 2: Guard Service Extraction, Phase 3: App State Decomposition, Phase 4: Gateway Decomposition (+20 more)

### Community 172 - "agents"
Cohesion: 0.18
Nodes (12): default, items, type, WhenClause, default, items, type, type (+4 more)

### Community 173 - "PostgresFinancialAdapter"
Cohesion: 0.07
Nodes (29): financial_store_error(), PostgresFinancialAdapter, ApprovalRequirement, Arc, CreateFinancialActionRequest, CreateFinancialMandateRequest, DateTime, FinancialActionListResponse (+21 more)

### Community 174 - "in_scope"
Cohesion: 0.18
Nodes (11): properties, type, AgentScope, default, items, type, default, items (+3 more)

### Community 175 - "properties"
Cohesion: 0.08
Nodes (25): type, type, type, type, properties, checked_input_excerpt, checked_output_excerpt, constraints (+17 more)

### Community 176 - "definitions"
Cohesion: 0.18
Nodes (11): enum, type, definitions, Action, MatchClause, Matcher, Severity, anyOf (+3 more)

### Community 178 - "package.json"
Cohesion: 0.07
Nodes (28): dependencies, fumadocs-core, fumadocs-mdx, fumadocs-openapi, fumadocs-ui, next, react, react-dom (+20 more)

### Community 179 - "AgentTone"
Cohesion: 0.20
Nodes (10): properties, required, type, AgentTone, default, items, type, forbidden (+2 more)

### Community 180 - "route.ts"
Cohesion: 0.73
Nodes (3): GET(), authSignOutRedirectUrl(), isAuthSignOutGet()

### Community 181 - "definitions"
Cohesion: 0.12
Nodes (16): definitions, RedactionMode, RedactionStatus, Severity, Tier, TierStatus, enum, type (+8 more)

### Community 182 - "TriggeredPolicy"
Cohesion: 0.20
Nodes (10): TriggeredPolicy, type, id, reason, severity, type, $ref, properties (+2 more)

### Community 185 - "proxy-helpers.ts"
Cohesion: 0.13
Nodes (19): PATCH(), GET(), POST(), GET(), POST(), PATCH(), GET(), POST() (+11 more)

### Community 190 - "financial.rs"
Cohesion: 0.13
Nodes (50): ApprovalRequirement, CounterpartyRef, CreateFinancialActionRequest, CreateFinancialMandateRequest, CreateFinancialPolicyRequest, EvidenceRef, FinancialAction, FinancialActionDecision (+42 more)

### Community 191 - "UserRepo"
Cohesion: 0.20
Nodes (14): find_user_by_oauth(), find_user_by_username_conn(), map_insert_err(), normalize_provider(), DbConnection, DbPool, Error, Option (+6 more)

### Community 194 - "pull_request_template.md"
Cohesion: 0.25
Nodes (7): 🔁 Cross-cutting concerns, 👀 Reviewer prompt, 🧩 SDK-parity checklist, 📝 Summary, ✅ Test plan, 🧭 Type of change, 🎨 UI Changes

### Community 195 - "WorkspaceInvite.ts"
Cohesion: 0.14
Nodes (12): CreateInviteRequest, CreateInviteResponse, InviteListResponse, InviteStatus, MemberListResponse, MyWorkspace, MyWorkspacesResponse, RFC-3339 (+4 more)

### Community 196 - "validation.rs"
Cohesion: 0.19
Nodes (23): Box, document_family(), FamilyTag, is_yaml_content_type(), parse_document(), parse_policy(), parse_policy_body(), ParsedPolicyBody (+15 more)

### Community 198 - "definitions"
Cohesion: 0.17
Nodes (11): type, definitions, AgentAuthority, KnowledgeSourceKind, description, enum, type, required (+3 more)

### Community 199 - "default"
Cohesion: 0.33
Nodes (6): agents, channels, domains, when, allOf, default

### Community 200 - "PolicyRepo"
Cohesion: 0.16
Nodes (16): AnyPolicyRow, cache_key(), PolicyRepo, PolicyRow, Arc, Cache, DbConnection, DbPool (+8 more)

### Community 201 - "workflow_requirements"
Cohesion: 0.20
Nodes (10): $ref, default, items, type, knowledge_sources, workflow_requirements, default, description (+2 more)

### Community 202 - "compilerOptions"
Cohesion: 0.10
Nodes (20): compilerOptions, allowSyntheticDefaultImports, esModuleInterop, exactOptionalPropertyTypes, forceConsistentCasingInFileNames, isolatedModules, lib, module (+12 more)

### Community 203 - "channels"
Cohesion: 0.40
Nodes (5): default, items, type, $ref, channels

### Community 204 - "policy.schema.json"
Cohesion: 0.40
Nodes (4): required, $schema, title, type

### Community 205 - "lint-web-backend-only.sh"
Cohesion: 0.60
Nodes (5): is_server_side(), scan_browser_only_rules(), scan_file(), scan_provider_sdks_anywhere(), lint-web-backend-only.sh script

### Community 206 - "Client"
Cohesion: 0.08
Nodes (30): Client, FinancialOperation, CounterpartyRef, CreateFinancialActionRequest, CreateFinancialMandateRequest, CreateFinancialPolicyRequest, EvidenceRef, FinancialActionDecisionReceipt (+22 more)

### Community 208 - "lint-no-internal-imports.sh"
Cohesion: 0.70
Nodes (4): scan_python(), scan_rust(), scan_typescript(), lint-no-internal-imports.sh script

### Community 210 - "FamilyPolicy"
Cohesion: 0.12
Nodes (30): AnyPolicy, ApprovalPolicy, ApprovalWhen, default_block_action(), default_escalate_action(), default_severity(), FamilyPolicy, FinancialPolicy (+22 more)

### Community 211 - "budget_alerts.rs"
Cohesion: 0.10
Nodes (25): crossed(), deliver_firing(), evaluate_spend_alerts(), firing_payload(), min_window_caps(), process_spend(), RecordBudgetAlertFiring, Arc (+17 more)

### Community 213 - "PostgresPolicyAdapter"
Cohesion: 0.14
Nodes (15): policy_action(), policy_summary_from_row(), PostgresPolicyAdapter, Action, Arc, EntityVersionDetail, EntityVersionListResponse, PolicyDocument (+7 more)

### Community 214 - "MemoryRunStore"
Cohesion: 0.13
Nodes (19): MemoryRunStore, p95_latency(), CreateRunEventRequest, CreateRunRequest, HashMap, Option, Result, RunEventSummary (+11 more)

### Community 216 - "RedteamJobStore"
Cohesion: 0.10
Nodes (36): RedteamJobStore, Send, Sync, DispatchConfig, DispatchJob, DispatchOutcome, drive(), is_cancelled() (+28 more)

### Community 220 - "CheckerRun.ts"
Cohesion: 0.17
Nodes (10): CheckerFindingEvidence, CheckerRun, DataHandlingMode, EnforcementMode, EnvironmentCheckerModes, RFC-3339, UpdateEnvironmentCheckerModesRequest, UpdateWorkspaceSettingsRequest (+2 more)

### Community 222 - "validation.rs"
Cohesion: 0.14
Nodes (25): accepts_max_only_value_limit(), accepts_valid_value_limit(), metadata(), rejects_blank_allowed_source_id(), rejects_blank_approver_role(), rejects_blank_tool_name(), rejects_duplicate_param_paths(), rejects_empty_param_path() (+17 more)

### Community 223 - "dashboard.rs"
Cohesion: 0.18
Nodes (23): ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse, CreateApiKeyRequest, CreateApiKeyResponse, CreateWorkspaceEnvironmentRequest, DashboardApiKey, DataHandlingMode (+15 more)

### Community 224 - "policy.rs"
Cohesion: 0.12
Nodes (30): AiEditRequest, AiEditResponse, default_policy_family(), EntityVersionDetail, EntityVersionListResponse, EntityVersionSummary, GuardrailGenerateResponse, GuardrailListResponse (+22 more)

### Community 225 - "redteam_runner.rs"
Cohesion: 0.13
Nodes (35): empty_json_object(), RedteamRunnerContract, HashMap, Option, String, Value, Vec, runner_attack_surface_is_default() (+27 more)

### Community 228 - "PostgresAnalyticsAdapter"
Cohesion: 0.15
Nodes (13): AnalyticsRepo, analytics_store_error(), PostgresAnalyticsAdapter, AnalyticsDashboardView, AnalyticsFacetCatalogResponse, AnalyticsQueryRequest, AnalyticsQueryResponse, Arc (+5 more)

### Community 229 - "value_limit.rs"
Cohesion: 0.17
Nodes (23): absent_param_is_skipped(), allows_amount_at_max_boundary(), allows_amount_at_min_boundary(), allows_amount_under_max(), blocks_when_amount_below_min(), blocks_when_amount_exceeds_max(), bound_finding(), escalates_when_value_is_not_an_integer() (+15 more)

### Community 230 - "metrics.rs"
Cohesion: 0.15
Nodes (23): AnalyticsChartType, AnalyticsDimension, AnalyticsFilter, AnalyticsMetric, BTreeSet, default_chart_type(), dimension_label(), fact_values() (+15 more)

### Community 231 - "RunState"
Cohesion: 0.18
Nodes (26): resolve_environment_id(), HeaderMap, Response, Result, String, create_run(), create_run_event(), get_run() (+18 more)

### Community 232 - "tool_metadata.rs"
Cohesion: 0.25
Nodes (25): app(), delete_then_get_returns_404(), disabled_tool_resolves_as_unregistered(), dotted_tool_name_routes_in_path(), duplicate_param_path_returns_422(), event_request(), event_trace_carries_resolved_metadata(), event_trace_carries_unregistered_resolution() (+17 more)

### Community 233 - "EscalationRepo"
Cohesion: 0.14
Nodes (16): EscalationRepo, EscalationRow, DateTime, DbConnection, DbPool, Debug, Duration, Formatter (+8 more)

### Community 234 - "RouterConfig"
Cohesion: 0.15
Nodes (16): BudgetConfig, ConfigError, empty_budgets_section_uses_default(), ProviderConfig, ProviderTarget, round_trips_sample_config(), RouteConfig, RouterConfig (+8 more)

### Community 237 - "package.json"
Cohesion: 0.08
Nodes (24): bin, trustloopguard-mcp-server, dependencies, @modelcontextprotocol/sdk, @trustloopguard/sdk, zod, description, devDependencies (+16 more)

### Community 238 - "entrypoint"
Cohesion: 0.20
Nodes (11): blocked_reply(), entrypoint(), escalated_reply(), HealthcareAgent, log_guardrail(), Agent, Decision, JobContext (+3 more)

### Community 239 - "GatewayStoreError"
Cohesion: 0.19
Nodes (13): GatewayStoreError, MemoryGatewayStore, EnforcementProfile, EnforcementProfilePatch, GatewayProviderConnection, GatewayRoute, GatewayRoutePatch, NewEnforcementProfile (+5 more)

### Community 242 - "engine.rs"
Cohesion: 0.14
Nodes (14): Engine, Arc, Self, Vec, OrchestrateConfig, Default, Duration, Self (+6 more)

### Community 244 - "README.md"
Cohesion: 0.10
Nodes (19): Backend tests, Built for, Choose your integration path, Contributing, Decision outcomes, Development setup, Documentation diagrams, Features (+11 more)

### Community 246 - "plan.rs"
Cohesion: 0.05
Nodes (64): agent_disambiguator(), core_path(), core_vector(), delete_plan(), generate_static_policies(), id_slug(), list_plans(), plan_attack_vectors() (+56 more)

### Community 247 - "escalation.rs"
Cohesion: 0.12
Nodes (36): default_retry_policy_is_five_attempts(), deliver_one(), delivery_loop(), EscalationConfig, EscalationPayload, persist_pending(), RetryPolicy, Arc (+28 more)

### Community 248 - "TeamStoreError"
Cohesion: 0.09
Nodes (30): generate_memory_token(), MemoryTeamState, MemoryTeamStore, AddMemberOutcome, MyWorkspace, Option, Result, RwLock (+22 more)

### Community 249 - "RetryConfig"
Cohesion: 0.16
Nodes (22): RateLimited, SdkError, Retry policy. Defaults match `tl-sdk-rust`'s `RetryConfig::default`., Compute the delay before the next attempt, or ``None`` to stop.          Mirrors, RetryConfig, _invalid(), output_event(), GuardEvent (+14 more)

### Community 250 - "KnowledgeRepo"
Cohesion: 0.15
Nodes (18): KnowledgeFileRow, KnowledgeRepo, KnowledgeSourceRow, NewKnowledgeFile, NewKnowledgeSource, DateTime, DbConnection, DbPool (+10 more)

### Community 252 - "writer.rs"
Cohesion: 0.13
Nodes (25): build_trace_payload(), event(), flush(), DbPool, Decision, Default, Duration, GuardEvent (+17 more)

### Community 253 - "package.json"
Cohesion: 0.08
Nodes (23): dependencies, geist, next, react, react-dom, @t3-oss/env-nextjs, zod, devDependencies (+15 more)

### Community 254 - "run_summary"
Cohesion: 0.13
Nodes (20): event_summary(), p95(), Option, Result, RunEventSummary, RunSummary, Vec, run_summary() (+12 more)

### Community 255 - "DashboardAdminStoreError"
Cohesion: 0.17
Nodes (18): DashboardAdminStoreError, memory_api_key_to_wire(), MemoryApiKeyRecord, MemoryApiKeyStore, MemorySettingsStore, normalize_ids(), DashboardApiKey, EnvironmentCheckerModes (+10 more)

### Community 256 - "Result"
Cohesion: 0.19
Nodes (11): insert_policy_version(), PolicyRepo, DbConnection, Option, PolicyFamily, PolicyRow, Result, String (+3 more)

### Community 257 - ".prettierrc.json"
Cohesion: 0.17
Nodes (11): arrowParens, bracketSameLine, bracketSpacing, endOfLine, printWidth, quoteProps, semi, singleQuote (+3 more)

### Community 258 - "dependencies"
Cohesion: 0.09
Nodes (23): dependencies, class-variance-authority, clsx, @dnd-kit/core, @dnd-kit/sortable, @dnd-kit/utilities, lucide-react, @monaco-editor/react (+15 more)

### Community 259 - "MokaCache"
Cohesion: 0.18
Nodes (14): disabled_cache_never_stores(), fake_decision(), miss_returns_none(), MokaCache, put_overwrites_existing_key(), put_then_get_returns_value(), Cache, Decision (+6 more)

### Community 260 - "check.ts"
Cohesion: 0.15
Nodes (27): assertProviderSuccess(), providerRequest(), restoreEnv(), testProviderAuthAndSimulation(), testStripeSafetyAndMapping(), handleHttpRequest(), handleProviderPayment(), providerApiKey() (+19 more)

### Community 264 - "page.tsx"
Cohesion: 0.04
Nodes (65): ChangePasswordCardProps, AuthScreenProps, BrandRailProps, VERDICTS, buildRetryUrl(), createWorkspace(), firstParam(), readOptionalField() (+57 more)

### Community 265 - "event"
Cohesion: 0.09
Nodes (43): allows_trusted_public_flow_to_external_sink(), blocks_private_source_flowing_to_external_sink(), blocks_untrusted_controlled_high_impact_action(), emits_both_rules_when_both_violated(), escalates_dangling_provenance_source_ids(), escalates_missing_provenance_on_high_impact_action(), escalates_unattributed_provenance_paths(), escalates_unknown_trust_control_on_high_impact_action() (+35 more)

### Community 267 - "tool.rs"
Cohesion: 0.13
Nodes (23): AllowedSource, ApprovalRule, LimitAction, ParamLimit, ParamRole, ParamSpec, AllowedSource, ApprovalRule (+15 more)

### Community 268 - "finalize_gateway_response"
Cohesion: 0.22
Nodes (22): finish_completed(), handle_output_enforcement(), handle_regeneration(), handle_rewrite_output(), output_safe_response(), OutputEnforcement, Decision, Option (+14 more)

### Community 269 - "label_policy.rs"
Cohesion: 0.24
Nodes (23): app(), delete_then_get_returns_not_found(), disabled_policy_listed_but_not_resolved(), disabled_policy_not_applied_at_runtime(), event_path_decision_unchanged_with_label_policies_configured(), event_request(), invalid_origin_path_rejected(), json_request() (+15 more)

### Community 271 - "hero.tsx"
Cohesion: 0.12
Nodes (8): HeroCard(), Hero(), LEDGER, LedgerRecord, VERDICT_COLOR, ITEMS, TrustBand(), TrustItem

### Community 273 - "properties"
Cohesion: 0.13
Nodes (15): type, RedactionInfo, items, type, type, $ref, context_redacted, entities (+7 more)

### Community 276 - "harden_job"
Cohesion: 0.13
Nodes (29): candidate_source(), ClassGroup, harden_job(), is_control(), load_workflow_requirements(), match_has_semantic(), matcher_is_semantic(), policy_has_semantic_matcher() (+21 more)

### Community 277 - "gateway.rs"
Cohesion: 0.23
Nodes (22): CreateEnforcementProfileRequest, CreateGatewayProviderConnectionRequest, CreateGatewayRouteRequest, EnforcementProfile, EnforcementProfileListResponse, FailMode, GatewayCredentialStatus, GatewayInputAction (+14 more)

### Community 278 - "writer.rs"
Cohesion: 0.31
Nodes (14): batch_size_triggers_flush(), caller_send_is_non_blocking_under_load(), event_evidence_round_trips_in_payload(), fake_decision(), fresh_pool(), graceful_shutdown_flushes_remaining(), interval_flushes_partial_batch(), ContainerAsync (+6 more)

### Community 279 - "memory.rs"
Cohesion: 0.17
Nodes (14): lock_error(), MemoryEnforcementProfile, MemoryGatewayRoute, MemoryGatewayStore, MemoryProviderConnection, EnforcementProfile, GatewayProviderConnection, GatewayRoute (+6 more)

### Community 280 - "traces.rs"
Cohesion: 0.10
Nodes (29): ChannelTraceStore, list_traces(), MemoryTraceStore, read_query_param(), Arc, DateTime, Decision, GuardEvent (+21 more)

### Community 281 - "EnvironmentRepo"
Cohesion: 0.18
Nodes (14): clear_default(), environment_to_wire(), EnvironmentRepo, CreateWorkspaceEnvironmentRequest, DbConnection, DbPool, Debug, Formatter (+6 more)

### Community 282 - "retry.rs"
Cohesion: 0.21
Nodes (18): caps_per_retry_delay_at_max_delay(), honors_retry_after_when_longer_than_jittered(), ignores_retry_after_when_jitter_already_longer(), invalid(), jitter_fraction_clamps_to_unit_interval(), non_retriable_errors_stop_immediately(), rate_limited(), retries_unavailable_with_exponential_backoff() (+10 more)

### Community 283 - "Contributing to TrustLoopGuard"
Cohesion: 0.25
Nodes (8): Commit style, Contributing to TrustLoopGuard, Development setup, License, Proposing changes, Pull request checklist, Reporting bugs, The three SDK-driven rules

### Community 284 - "RedteamPlanRepo"
Cohesion: 0.17
Nodes (16): parse_uuid(), plan_response(), PlanBody, RedteamPlanRepo, AttackVector, DbConnection, DbPool, Debug (+8 more)

### Community 285 - "knowledge.rs"
Cohesion: 0.18
Nodes (15): knowledge_kind_text(), knowledge_row_to_document(), parse_knowledge_kind(), parse_knowledge_status(), PostgresKnowledgeAdapter, Arc, CreateKnowledgeSourceRequest, KnowledgeSourceDocument (+7 more)

### Community 287 - "definitions"
Cohesion: 0.11
Nodes (18): definitions, RunnerAttackSurface, RunnerRunMode, RunnerSessionEvent, RunnerStatus, description, enum, type (+10 more)

### Community 288 - "redteam-core.ts"
Cohesion: 0.13
Nodes (17): ALLOWED_AGENT_HOSTS, REDTEAM_PROFILES, RedteamCase, redteamCaseSchema, redteamLlmSchema, redteamOutcomeSchema, redteamProfileSchema, RedteamReport (+9 more)

### Community 291 - "handlers.rs"
Cohesion: 0.22
Nodes (29): api_error_response(), budget_alert_error_response(), BudgetAlertApiState, clean_optional(), create_budget_alert(), delete_budget_alert(), list_budget_alert_firings(), list_budget_alerts() (+21 more)

### Community 292 - "api_keys.rs"
Cohesion: 0.21
Nodes (20): ApiKeyListRow, api_key_row_to_wire(), ApiKeyAuthRecord, ApiKeyRecord, DashboardAdminRepo, ensure_all_keys_exist(), environment_slug(), load_api_key_rows() (+12 more)

### Community 295 - "insert_existing_workspace_member"
Cohesion: 0.10
Nodes (28): AddMemberOutcome, insert_existing_workspace_member(), load_usernames(), member_row_to_wire(), DbConnection, HashMap, Result, String (+20 more)

### Community 296 - "aggregate"
Cohesion: 0.22
Nodes (22): BlockSignal, Verdict, JudgeOutcomes, JudgeResult, LlmRouter, run_judges(), aggregate(), apply_authority_verdict() (+14 more)

### Community 297 - "State"
Cohesion: 0.27
Nodes (31): State, FinancialState, approve_action(), create_action(), create_mandate(), create_policy(), deny_action(), execute_action() (+23 more)

### Community 298 - "PostgresHumanReviewAdapter"
Cohesion: 0.16
Nodes (13): human_review_store_error(), PostgresHumanReviewAdapter, Arc, CreateHumanReviewEventRequest, HumanReviewAnalyticsFilter, HumanReviewAnalyticsResponse, HumanReviewEvent, Option (+5 more)

### Community 300 - "store.rs"
Cohesion: 0.14
Nodes (19): EnforcementProfilePatch, GatewayRoutePatch, NewEnforcementProfile, NewGatewayProviderConnection, NewGatewayRoute, ProviderConnectionPatch, ProviderConnectionSecret, ResolvedGatewayRoute (+11 more)

### Community 301 - "event_service.rs"
Cohesion: 0.15
Nodes (20): event(), execute_event_submission(), rejects_duplicate_source_ids(), rejects_empty_agent_and_operation(), rejects_oversized_parameters(), rejects_oversized_provenance(), rejects_too_many_sources(), Decision (+12 more)

### Community 302 - ".create_event"
Cohesion: 0.15
Nodes (16): HumanReviewRepo, CreateHumanReviewEventRequest, DbConnection, DbPool, Debug, Formatter, HashMap, HumanReviewEvent (+8 more)

### Community 303 - "Web UI Conventions"
Cohesion: 0.08
Nodes (26): API, API, API, API, BatchActionBar, CopyBlock, Current adopters, Dashboard API Calls (+18 more)

### Community 304 - "MemoryLlmUsageStore"
Cohesion: 0.16
Nodes (23): duplicate_request_id_is_a_noop(), event(), event_matches(), event_with_cost(), grouped_model_usage_preserves_zero_cost_undercount_signal(), grouped_usage_by_day_uses_utc_date_key(), grouped_usage_folds_by_principal_and_model(), MemoryLlmUsageStore (+15 more)

### Community 306 - "enforcement.rs"
Cohesion: 0.18
Nodes (10): CheckerFindingEvidence, CheckerFindingEvidence, CheckerRun, EnforcementMode, Option, Severity, String, Vec (+2 more)

### Community 307 - "properties"
Cohesion: 0.09
Nodes (22): type, default, type, type, default, type, default, type (+14 more)

### Community 308 - "properties"
Cohesion: 0.09
Nodes (22): type, default, type, type, type, attack, caseId, landed (+14 more)

### Community 309 - "package.json"
Cohesion: 0.10
Nodes (21): default, description, devDependencies, typescript, vitest, exports, files, import (+13 more)

### Community 310 - "SourceLabelEvidence"
Cohesion: 0.07
Nodes (31): properties, required, type, $ref, confidentiality, integrity, trust, AllowedSource (+23 more)

### Community 311 - "LlmUsageRepo"
Cohesion: 0.15
Nodes (19): LlmUsageBucketRow, LlmUsageEventFilter, LlmUsageGroupBy, LlmUsageRepo, NewLlmUsageEventParams, DateTime, DbConnection, DbPool (+11 more)

### Community 312 - "EnforcementProfile.ts"
Cohesion: 0.27
Nodes (9): CreateEnforcementProfileRequest, EnforcementProfile, EnforcementProfileListResponse, FailMode, GatewayInputAction, GatewayOutputAction, ResponseMode, RetentionMode (+1 more)

### Community 314 - "environment_deployments.rs"
Cohesion: 0.24
Nodes (14): ensure_all_policies_exist(), ensure_policy_exists(), load_any_deployment_records(), PolicyRepo, Arc, DbConnection, Option, PolicyFamily (+6 more)

### Community 315 - "fresh_pool"
Cohesion: 0.36
Nodes (7): create_event_auto_sequence_is_concurrency_safe(), create_event_rejects_invalid_input(), create_list_and_update_run(), fresh_pool(), ContainerAsync, DbPool, PostgresImage

### Community 316 - "knip.json"
Cohesion: 0.18
Nodes (10): ignore, ignoreBinaries, ignoreDependencies, ignoreFiles, ignoreIssues, apps/docs/source.config.ts, apps/web/components/ui/**, sdks/typescript/src/generated/** (+2 more)

### Community 317 - "Code of Conduct"
Cohesion: 0.33
Nodes (5): Attribution, Code of Conduct, Enforcement, Our Pledge, Our Standards

### Community 318 - "PostgresLabelPolicyAdapter"
Cohesion: 0.19
Nodes (16): label_policy_store_error(), origin_key(), PostgresLabelPolicyAdapter, Arc, Option, Origin, PolicyRepo, Result (+8 more)

### Community 319 - "GuardLogEvent"
Cohesion: 0.07
Nodes (56): GuardModeInput, OnAllowAsync, OnAllowSync, OnBlockAsync, OnBlockSync, OnErrorAsync, OnErrorSync, OnEscalateAsync (+48 more)

### Community 320 - "RunnerAttackVector"
Cohesion: 0.10
Nodes (21): RunnerAttackVector, description, type, description, type, goal, injectionPayload, sourcePath (+13 more)

### Community 321 - "decision.schema.json"
Cohesion: 0.40
Nodes (4): required, $schema, title, type

### Community 322 - "sdk.tsx"
Cohesion: 0.16
Nodes (12): CodeBlock(), CodeBlockProps, highlight(), KEYWORDS, LABELS, Lang, tokenize(), Mode (+4 more)

### Community 323 - "prepush-fast.sh"
Cohesion: 0.43
Nodes (5): add_package(), detect_base_ref(), ref_exists(), run(), prepush-fast.sh script

### Community 325 - "lib.rs"
Cohesion: 0.09
Nodes (25): EnforcementProfilePatch, GatewayProviderConnectionSecret, GatewayRepo, GatewayRoutePatch, ResolvedGatewayRoute, DbConnection, DbPool, EnforcementProfile (+17 more)

### Community 326 - "RedactedEntity"
Cohesion: 0.17
Nodes (12): format, minimum, type, RedactedEntity, type, count, entity_type, token (+4 more)

### Community 327 - "parse_retry_after"
Cohesion: 0.26
Nodes (10): B, Client, parse_retry_after(), Duration, F, HeaderMap, Option, Result (+2 more)

### Community 328 - "Write Your First Policy"
Cohesion: 0.20
Nodes (10): Choosing A Matcher, Choosing An Action, Common Fixes, Copy This Rule, Keep Reading, Local And Cloud Mode, Push And Pull, Validate It (+2 more)

### Community 329 - "RunStoreError"
Cohesion: 0.20
Nodes (13): RunStoreError, PostgresRunAdapter, Arc, CreateRunEventRequest, CreateRunRequest, Result, RunEventSummary, RunSummary (+5 more)

### Community 331 - "evaluate_financial_policies"
Cohesion: 0.15
Nodes (29): action_verdict(), compose(), evaluate_financial_policies(), financial_windowed_verdict(), per_action_verdicts(), Action, FinancialAction, I (+21 more)

### Community 332 - "StorageError"
Cohesion: 0.09
Nodes (39): AnalyticsRepo, clear_default(), ensure_view_exists(), AnalyticsDashboardView, CreateAnalyticsDashboardViewRequest, DbConnection, Result, UpdateAnalyticsDashboardViewRequest (+31 more)

### Community 333 - "tests.rs"
Cohesion: 0.27
Nodes (9): memory_store_delete_then_get_not_found(), memory_store_list_sorted(), memory_store_round_trip(), profile(), AgentProfile, validate_accepts_small_workflow_definition(), validate_rejects_empty_agent_id(), validate_rejects_empty_in_scope() (+1 more)

### Community 334 - "GuardEvent Redaction Spec"
Cohesion: 0.10
Nodes (19): 1. SDK-local redaction, 2. Customer-environment redaction service, 3. Server-side redaction, Acceptance Criteria, Deployment Modes, Goals, GuardEvent Redaction Spec, Hosted Cloud Behavior (+11 more)

### Community 335 - "authorize_workspace_admin"
Cohesion: 0.20
Nodes (19): authorize_api_key_management(), authorize_workspace_admin(), forwarded_user_id(), require_admin_role(), Arc, Extension, HeaderMap, Option (+11 more)

### Community 337 - ".check"
Cohesion: 0.18
Nodes (16): applies_on_memory_write_side_effect_even_for_other_kinds(), blocks_untrusted_memory_write(), escalates_unattributed_provenance_paths(), escalates_unverified_memory_write(), ignores_non_memory_events(), MemoryChecker, provenance(), GuardEvent (+8 more)

### Community 339 - "financial_actions.rs"
Cohesion: 0.22
Nodes (28): app(), app_for(), create_payment_connection(), financial_action_decision_receipt_explains_held_refund(), financial_action_decision_receipt_missing_action_returns_404(), financial_action_outcomes_record_and_list(), financial_actions_create_get_and_transition(), financial_actions_list_workspace_actions() (+20 more)

### Community 340 - "budget_alerts.rs"
Cohesion: 0.21
Nodes (28): absolute_threshold_fires_when_remaining_drops_to_value(), admin_request(), app_with_owner(), create_alert(), create_weekly_cap(), crud_round_trip_via_router(), delivery_tx(), disabled_config_stays_silent() (+20 more)

### Community 341 - "build_postgres_layer"
Cohesion: 0.07
Nodes (46): AgentStore, Send, Sync, Send, Sync, UserStore, BudgetAlertStore, Send (+38 more)

### Community 342 - "financial_actions_integration.rs"
Cohesion: 0.15
Nodes (26): action_body(), decision_receipt_body(), financial_action_helpers_encode_ids_and_parse_statuses(), financial_mandate_helpers_create_list_and_revoke(), financial_outcome_helpers_record_and_list(), financial_policy_body(), financial_policy_helpers_create_and_list_controls(), financial_policy_request() (+18 more)

### Community 343 - "team.rs"
Cohesion: 0.20
Nodes (15): CreateInviteRequest, CreateInviteResponse, CreateWorkspaceRequest, InviteListResponse, InviteStatus, MemberListResponse, MyWorkspace, MyWorkspacesResponse (+7 more)

### Community 344 - "FinancialExecutionResult"
Cohesion: 0.18
Nodes (20): FinancialExecutionError, FinancialExecutionResult, FinancialExecutor, PaymentHttpFinancialExecutor, provider_body(), recovery_status(), reversal_capability(), Arc (+12 more)

### Community 345 - ".create_event"
Cohesion: 0.18
Nodes (16): CreateRunEventRequest, Result, RunEventSummary, Vec, RunRepo, non_empty_string(), normalize_metadata(), parse_run_id() (+8 more)

### Community 347 - ".query"
Cohesion: 0.15
Nodes (12): AnalyticsRepo, AnalyticsQueryRequest, Result, validate_query(), AnalyticsQueryRequest, AnalyticsQueryResponse, DbConnection, DbPool (+4 more)

### Community 348 - "Channel"
Cohesion: 0.50
Nodes (4): description, enum, type, Channel

### Community 349 - "Security Policy"
Cohesion: 0.25
Nodes (7): Coordinated Disclosure, Reporting a Vulnerability, Scope, Security Policy, Supported Versions, What to expect, What to include

### Community 350 - "properties"
Cohesion: 0.13
Nodes (15): description, properties, required, type, CheckerFindingEvidence, type, type, failure_mode (+7 more)

### Community 351 - "RunnerDocumentTemplate"
Cohesion: 0.11
Nodes (19): type, type, RunnerDocumentTemplate, additionalProperties, type, type, default, type (+11 more)

### Community 352 - "TrustLoopGuard Hardening v2 — Attack-Grounded Policy Synthesis"
Cohesion: 0.11
Nodes (18): 1. Attack taxonomy → remediation substrate, 2. Synthesis pipeline, 3. Generalization (concrete → class), 4. Verify-before-recommend (loop closure), 5. LLM usage: synthesis-time vs runtime (two planes), Architecture, Background: how v1 hardening works, and why it can't generalize, Concept-doc / contract impact when this ships (+10 more)

### Community 353 - "GatewayState"
Cohesion: 0.08
Nodes (48): create_enforcement_profile(), list_enforcement_profiles(), patch_enforcement_profile(), CreateEnforcementProfileRequest, Extension, HeaderMap, Json, Option (+40 more)

### Community 354 - "components.json"
Cohesion: 0.11
Nodes (17): aliases, components, hooks, lib, ui, utils, iconLibrary, rsc (+9 more)

### Community 355 - "policy_repo.rs"
Cohesion: 0.29
Nodes (15): batch_set_enabled_is_atomic_for_missing_policy(), batch_set_enabled_updates_all_selected_policies(), fresh_repo(), list_enabled_filters_disabled_and_deleted(), missing_policy_returns_not_found(), ContainerAsync, PolicyRepo, PostgresImage (+7 more)

### Community 356 - ".start_run"
Cohesion: 0.12
Nodes (11): CreateRunRequest, RunStatus, RunSummary, UpdateRunRequest, Async variant of ``Client.start_run``., Async variant of ``Client.update_run``., Async variant of ``Client.finish_run``., Create a run grouping for subsequent ``check`` calls. (+3 more)

### Community 357 - "effective_checker_modes"
Cohesion: 0.19
Nodes (18): checker_run_evidence(), CheckerModes, CheckerRun, EnforcementMode, all_none_override_inherits_workspace_modes(), checker_modes(), effective_checker_modes(), no_override_inherits_workspace_modes() (+10 more)

### Community 358 - "severity"
Cohesion: 0.67
Nodes (3): severity, allOf, default

### Community 359 - "PolicyStore"
Cohesion: 0.20
Nodes (9): BudgetAlertRuntime, Sender, FinancialStore, Send, Sync, Self, PolicyStore, Send (+1 more)

### Community 360 - "LlmClient"
Cohesion: 0.29
Nodes (16): LlmClient, Send, Sync, build_budget(), build_provider(), build_providers(), build_routes(), ensure_provider_exists() (+8 more)

### Community 361 - "tests.rs"
Cohesion: 0.26
Nodes (13): missing_route_yields_http_error(), MockClient, no_fallback_propagates_primary_error(), over_budget_blocks_request_before_calling_provider(), primary_failure_falls_back_to_secondary(), primary_success_records_budget_and_skips_fallback(), Arc, AtomicUsize (+5 more)

### Community 362 - "embedder.rs"
Cohesion: 0.23
Nodes (13): cosine(), EmbedError, FastEmbedder, fnv1a(), mock_embedder_is_deterministic(), mock_embedder_normalises_to_unit(), Mutex, Result (+5 more)

### Community 363 - "require_approved_user"
Cohesion: 0.19
Nodes (14): forwarded_user_id(), require_approved_user(), Option, Request, Response, Result, Uuid, api_error() (+6 more)

### Community 364 - "read_filter"
Cohesion: 0.16
Nodes (17): parse_kind(), parse_status(), query_parts(), read_filter(), read_limit(), Item, Iterator, Option (+9 more)

### Community 365 - "sync-recipes.ts"
Cohesion: 0.20
Nodes (8): changed, escapeRegExp(), failures, Recipe, recipePaths, replaceBlock(), Snippet, Target

### Community 367 - "key.rs"
Cohesion: 0.24
Nodes (17): canonical_json(), context_object_key_order_does_not_affect_key(), different_domain_changes_key(), different_drafts_hash_differently(), for_check_request(), for_check_request_with_policy_scope(), identical_requests_hash_equal(), missing_domain_is_treated_as_default() (+9 more)

### Community 368 - "properties"
Cohesion: 0.11
Nodes (18): RunnerWorkflowPath, sinkCategory, sinkNode, sinkType, sourceCategory, sourceNode, sourceType, additionalProperties (+10 more)

### Community 369 - "HnswIndex"
Cohesion: 0.20
Nodes (17): cosine_similarity(), dim_mismatch_yields_empty_query(), empty_index_returns_empty_query(), HnswIndex, identical_vector_scores_one(), IndexHit, mock_embedder_round_trip_through_index(), orthogonal_vector_below_threshold() (+9 more)

### Community 371 - "OpenRouterClient"
Cohesion: 0.32
Nodes (7): OpenRouterClient, Client, Duration, Into, Result, Self, String

### Community 372 - "resolved_event"
Cohesion: 0.19
Nodes (16): ApprovalChecker, empty_roles_fall_back_to_generic_remediation(), escalates_when_tool_requires_approval(), metadata(), no_approval_rule_emits_nothing(), not_required_emits_nothing(), registry_reason_wins_over_generated_remediation(), remediation() (+8 more)

### Community 374 - "TokenBudget"
Cohesion: 0.18
Nodes (14): BudgetExceeded, BudgetState, exceeding_default_limit_errors(), HashMap, Into, Mutex, Result, Self (+6 more)

### Community 375 - "load_agent_str"
Cohesion: 0.12
Nodes (29): load_agent_str(), AgentProfile, Result, loads_committed_fixture_acme_support_v3(), parses_full_featured_profile(), parses_minimal_profile(), parses_web_knowledge_source_metadata(), rejects_duplicate_knowledge_source_ids() (+21 more)

### Community 376 - "monitoring_integration.rs"
Cohesion: 0.25
Nodes (16): allow_decision(), caller_explicit_session_is_never_overwritten(), client_without_monitoring_sends_no_session_id(), event(), mock_post(), monitoring_client_tags_submitted_events_with_session(), one_shot_retry(), record_event_delivers_without_blocking() (+8 more)

### Community 377 - "validation.rs"
Cohesion: 0.33
Nodes (9): CreateRunEventRequest, CreateRunRequest, Result, UpdateRunRequest, Value, validate_create_run(), validate_create_run_event(), validate_metadata() (+1 more)

### Community 378 - "mod.rs"
Cohesion: 0.11
Nodes (20): JobCounts, PublicReportState, RedteamAttackRecordFilter, RedteamJobListFilter, Arc, Option, String, allows_up_to_max_then_blocks() (+12 more)

### Community 379 - "guardrails.rs"
Cohesion: 0.35
Nodes (12): build_app(), delete_agent_cascades_to_owned_policies(), generate_for_missing_agent_is_404(), generate_persists_each_draft_disabled_and_returns_them(), generate_without_system_prompt_is_422(), list_for_unknown_agent_returns_empty(), list_returns_policies_scoped_to_agent(), read_body() (+4 more)

### Community 380 - "financial_repo.rs"
Cohesion: 0.18
Nodes (21): approval_requests_are_tenant_scoped_and_newest_first(), create_action_is_idempotent_and_tenant_scoped(), fresh_pool(), list_actions_is_tenant_scoped_and_newest_first(), mandate_request(), mandates_create_list_and_revoke_are_tenant_scoped(), outcome(), outcomes_append_and_list_by_action_without_affecting_spend() (+13 more)

### Community 381 - "client.ts"
Cohesion: 0.05
Nodes (41): ActiveRun, ActiveRunContext, browserRunContext(), buildFinancialOperationRequest(), cleanFinancialOperationField(), FinancialOperationRunOptions, ListTracesOptions, runContext() (+33 more)

### Community 382 - "Crates"
Cohesion: 0.12
Nodes (17): Adding a new crate, Crates, Current Boundary Decisions, Dependency graph, `tl-cache` — decision cache, `tl-cli` — operator command line, `tl-codegen` — derived-artifact generator, `tl-core` — the type backbone (+9 more)

### Community 383 - "redteam-runner.schema.json"
Cohesion: 0.12
Nodes (16): description, $ref, $ref, $ref, $ref, properties, dispatch, handle (+8 more)

### Community 384 - "test_events.py"
Cohesion: 0.24
Nodes (14): TrustLoopGuard Python SDK.  Public surface:     Client          — HTTP client fo, Retry policy for the TrustLoopGuard Python SDK.  Mirrors `tl-sdk-rust`'s `RetryC, default_allow_decision(), GuardEvent, submit_event tests: typed round trip + error mapping, sync and async., run_event_summary(), run_summary(), send_email_event() (+6 more)

### Community 385 - "main.rs"
Cohesion: 0.26
Nodes (14): Args, main(), normalize_typescript(), patch_openapi_label_policy_upsert(), render_pydantic(), repo_root(), Option, Path (+6 more)

### Community 387 - "docs-auth.ts"
Cohesion: 0.22
Nodes (11): POST(), redirectTo(), POST(), redirectTo(), UnlockPage(), UnlockPageProps, createDocsAuthToken(), safeDocsRedirectPath() (+3 more)

### Community 388 - "scripts"
Cohesion: 0.05
Nodes (36): dependencies, openai, pdfjs-dist, @trustloopguard/sdk, yaml, description, devDependencies, tsx (+28 more)

### Community 390 - "human_review.rs"
Cohesion: 0.28
Nodes (15): CreateHumanReviewEventRequest, HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary, HumanReviewEvent, HumanReviewEventListResponse, HumanReviewGroupRow, HumanReviewOutcome, HumanReviewOutcomeCounts (+7 more)

### Community 391 - ".from_response"
Cohesion: 0.22
Nodes (12): body_with_unknown_code_falls_back_to_status(), carries_retry_after_for_rate_limit(), empty_body_500_synthesizes_internal_error(), falls_back_to_status_when_body_unrecognized(), parses_canonical_body_to_typed_variant(), ApiError, ApiErrorCode, Duration (+4 more)

### Community 392 - "put_llm_price"
Cohesion: 0.19
Nodes (21): api_error_response(), delete_llm_price(), list_llm_pricing(), LlmPricingState, price_row(), put_llm_price(), ApiErrorCode, Arc (+13 more)

### Community 393 - "Common Workflows"
Cohesion: 0.33
Nodes (5): Add A New Runtime Capability, Available Guides, Build A Demo, Common Workflows, Guard An Agent Reply

### Community 394 - "main.rs"
Cohesion: 0.18
Nodes (19): generate_guardrails(), list_guardrails(), GuardrailGenerateResponse, GuardrailListResponse, Option, Result, String, run_agents() (+11 more)

### Community 395 - "input.tsx"
Cohesion: 0.09
Nodes (22): event(), FormError(), FormErrorProps, Spinner(), SpinnerProps, CredentialsFormProps, OAuthButtons(), OAuthButtonsProps (+14 more)

### Community 396 - "UserStoreError"
Cohesion: 0.26
Nodes (9): MemoryUserStore, HashMap, Result, RwLock, Self, String, UserRecord, Uuid (+1 more)

### Community 399 - "properties"
Cohesion: 0.12
Nodes (16): $ref, RunnerDispatch, anyOf, $ref, type, attackSurface, documentTemplate, mode (+8 more)

### Community 400 - "ParamLimit"
Cohesion: 0.12
Nodes (16): ParamLimit, description, format, type, description, format, type, allOf (+8 more)

### Community 403 - "ParamLimit"
Cohesion: 0.12
Nodes (16): ParamLimit, description, format, type, description, format, type, allOf (+8 more)

### Community 404 - "RedteamDispatchRequest.ts"
Cohesion: 0.18
Nodes (9): AttackVector, RedteamAttackSurface, RedteamDispatchRequest, RedteamDocumentTemplate, RedteamPlanListResponse, RedteamPlanResponse, RFC-3339, RedteamRunMode (+1 more)

### Community 405 - "Client"
Cohesion: 0.18
Nodes (10): Client, ApiError, Into, Option, RetryConfig, Self, String, synthesize_api_error() (+2 more)

### Community 407 - "AnthropicGatewayProvider"
Cohesion: 0.07
Nodes (34): AnthropicGatewayProvider, Client, EnforcementProfile, GatewayProviderConnection, Result, String, Value, GatewayProvider (+26 more)

### Community 408 - "page.tsx"
Cohesion: 0.10
Nodes (22): Ascii(), ASCII_ART, AsciiName, CountUp(), CountUpProps, Cta(), Eyebrow(), How() (+14 more)

### Community 409 - "router"
Cohesion: 0.12
Nodes (29): build_policy_draft_llm(), router(), Arc, Option, memory_app_state(), analytics_catalog_query_and_saved_views_round_trip(), analytics_endpoints_are_protected_by_bearer_auth(), internal_bearer_analytics_requires_forwarded_workspace_member() (+21 more)

### Community 410 - "run.rs"
Cohesion: 0.31
Nodes (16): CreateRunEventRequest, CreateRunRequest, Option, String, TraceSummary, Value, Vec, RunDetail (+8 more)

### Community 411 - "github.ts"
Cohesion: 0.19
Nodes (8): formatStars(), GitHubStarLink(), NavActions(), NavActionsProps, Nav(), NAV_LINKS, getStarCount(), RepoSummary

### Community 412 - "Event Engine"
Cohesion: 0.13
Nodes (15): Checkers And Enforcement Modes, Collection Points, Compatibility Rules, Contract Vocabulary, Current Runtime Flow, Direct ingestion, Event Engine, Gateway (low fidelity) (+7 more)

### Community 414 - "Policy YAML Reference"
Cohesion: 0.13
Nodes (15): `action`, `description`, `id`, `literal`, `match`, Matchers, Policy YAML Reference, `regex` (+7 more)

### Community 415 - "LabelPolicyStoreError"
Cohesion: 0.23
Nodes (12): LabelPolicyStoreError, MemoryLabelPolicyStore, origin_key(), HashMap, Origin, Result, RwLock, Self (+4 more)

### Community 416 - "compilerOptions"
Cohesion: 0.14
Nodes (13): compilerOptions, allowJs, exactOptionalPropertyTypes, incremental, jsx, lib, noEmit, paths (+5 more)

### Community 417 - "verify_candidate"
Cohesion: 0.22
Nodes (17): candidate_that_false_blocks_a_control_does_not_pass(), candidate_that_misses_a_variant_does_not_pass(), fires(), output_event(), policy(), regex_candidate_verifies_without_a_judge(), GuardEvent, MatchClause (+9 more)

### Community 418 - "PostgresUserAdapter"
Cohesion: 0.22
Nodes (10): PostgresUserAdapter, Arc, Result, Self, UserRecord, Uuid, user_record_from_row(), user_store_create_error() (+2 more)

### Community 419 - "types.ts"
Cohesion: 0.12
Nodes (34): customerBackendState(), ensureOrderDatabase(), findOrder(), listOrders(), listRefunds(), nullableTextValue(), numberValue(), openDatabase() (+26 more)

### Community 421 - "event_summary"
Cohesion: 0.22
Nodes (11): event_summary(), parse_reason_codes(), HumanReviewEvent, Result, String, Value, Vec, outcome_text() (+3 more)

### Community 422 - "policy.rs"
Cohesion: 0.38
Nodes (11): decode_policy_response(), load_policy_file(), pull_policy(), push_policy(), Option, PathBuf, PolicyDocument, Response (+3 more)

### Community 423 - "seal_key_material"
Cohesion: 0.21
Nodes (12): build_seal_key(), Option, Result, String, seal_key_config_requires_secret_without_explicit_dev_override(), seal_key_material(), unseal_provider_key(), env_filter() (+4 more)

### Community 424 - "properties"
Cohesion: 0.10
Nodes (20): type, Principal, type, properties, required, type, agent_id, environment_id (+12 more)

### Community 425 - "LlmPricingStoreError"
Cohesion: 0.18
Nodes (11): LlmPricingStoreError, MemoryLlmPricingStore, BTreeMap, Option, Result, RwLock, Self, String (+3 more)

### Community 426 - "page.tsx"
Cohesion: 0.15
Nodes (8): { GET }, APIPage, MediaBody, scalarToYaml(), toYaml(), yamlMediaAdapter, openapi, source

### Community 427 - "build_app"
Cohesion: 0.29
Nodes (13): build_app(), delete_then_get_returns_404(), delete_unknown_yields_404(), list_returns_all_agents(), missing_agent_yields_404(), read_body(), Response, Router (+5 more)

### Community 429 - "validation.rs"
Cohesion: 0.23
Nodes (13): clean_reason_codes(), non_empty_string(), normalize_metadata(), parse_uuid(), CreateHumanReviewEventRequest, Option, Result, String (+5 more)

### Community 430 - "check_pipeline.rs"
Cohesion: 0.35
Nodes (11): bench_check_async_50_policies_4kb(), bench_check_async_cache_hit(), bench_check_async_empty_default(), bench_check_sync_empty(), bench_check_sync_empty_4kb(), bench_check_sync_policy_block_4kb(), fifty_policies(), large_req() (+3 more)

### Community 431 - "definitions"
Cohesion: 0.04
Nodes (46): enum, type, definitions, Confidentiality, EnforcementMode, EventKind, Integrity, LabelBasis (+38 more)

### Community 432 - "ToolMetadataStoreError"
Cohesion: 0.34
Nodes (14): delete_tool_metadata(), get_tool_metadata(), list_tool_metadata(), Arc, HeaderMap, Json, Path, Response (+6 more)

### Community 433 - "Human Review Analytics Spec"
Cohesion: 0.14
Nodes (13): Acceptance Criteria, API Contract, Dashboard UX, Data Model, Definitions, Goals, Human Review Analytics Spec, Implementation Scope (+5 more)

### Community 434 - "WorkflowRequirement"
Cohesion: 0.14
Nodes (14): WorkflowRequirement, type, name, required_before, sensitive_steps, default, items, type (+6 more)

### Community 435 - "HumanReviewAnalyticsResponse.ts"
Cohesion: 0.21
Nodes (7): HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary, HumanReviewGroupRow, HumanReviewOutcomeCounts, HumanReviewPolicyRow, HumanReviewReasonRow, HumanReviewWorkflowStepRow

### Community 436 - "Decision.ts"
Cohesion: 0.19
Nodes (8): RedactedEntity, RedactionInfo, RedactionMode, RedactionStatus, Tier, TierResult, TierStatus, TriggeredPolicy

### Community 437 - "compilerOptions"
Cohesion: 0.15
Nodes (12): compilerOptions, allowJs, incremental, jsx, lib, noEmit, paths, plugins (+4 more)

### Community 438 - "devDependencies"
Cohesion: 0.15
Nodes (13): devDependencies, jsdom, tailwindcss, @tailwindcss/postcss, @testing-library/jest-dom, @testing-library/react, @testing-library/user-event, @types/node (+5 more)

### Community 439 - "LlmRouter"
Cohesion: 0.27
Nodes (9): JudgeKind, LlmRouter, ResolvedRoute, Arc, Debug, HashMap, Option, Self (+1 more)

### Community 440 - "seed-demo.ts"
Cohesion: 0.31
Nodes (12): createKnowledgeSource(), DemoAgentProfile, DemoKnowledgeSource, DemoToolMetadata, DemoTraceInput, enforceDemoGuardSettings(), main(), recordTrace() (+4 more)

### Community 441 - "compilerOptions"
Cohesion: 0.15
Nodes (12): compilerOptions, allowJs, incremental, jsx, lib, noEmit, paths, plugins (+4 more)

### Community 442 - "knowledge.rs"
Cohesion: 0.28
Nodes (12): CreateKnowledgeSourceRequest, DashboardKnowledgeSourceKind, KnowledgeFileInput, KnowledgeFileMetadata, KnowledgeSourceDocument, KnowledgeSourceFileResponse, KnowledgeSourceListResponse, KnowledgeSourceStatus (+4 more)

### Community 443 - "LlmUsageStoreError"
Cohesion: 0.16
Nodes (14): LlmUsageStoreError, llm_usage_store_error(), PostgresLlmUsageAdapter, Arc, DateTime, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageGroupBy (+6 more)

### Community 444 - "HardenCandidate.ts"
Cohesion: 0.24
Nodes (7): HardenCandidate, HardenCandidateOperation, HardenRejection, HardenRejectionReason, HardenResponse, RFC-3339, VerifyResult

### Community 445 - "LlmPricingRepo"
Cohesion: 0.17
Nodes (12): LlmPricingRepo, DbConnection, DbPool, Debug, Formatter, Option, Result, Self (+4 more)

### Community 446 - "lib.rs"
Cohesion: 0.26
Nodes (9): buffer_truncates_to_window(), continues_when_evaluator_allows(), interrupts_when_evaluator_flags_window(), F, Self, String, Verdict, StreamDecision (+1 more)

### Community 448 - "http.rs"
Cohesion: 0.27
Nodes (9): decode_typed_response(), resolve_api_key(), Option, Response, Result, String, T, server_url() (+1 more)

### Community 449 - "policy"
Cohesion: 0.21
Nodes (10): policy(), rejects_empty_override(), Confidentiality, Integrity, Option, Result, SourceLabelPolicy, String (+2 more)

### Community 450 - "build_app_state"
Cohesion: 0.16
Nodes (15): password_auth_enabled_from_env(), password_auth_enabled_from_values(), Option, build_app_state(), build_dispatch_worker(), build_escalation_worker(), build_llm_router(), load_policies() (+7 more)

### Community 451 - "request"
Cohesion: 0.13
Nodes (19): build_app(), create_json_policy_canonicalizes_source_yaml(), create_then_get_policy_round_trips_source_yaml(), batch_disable_missing_policy_does_not_partially_update(), batch_disable_updates_multiple_policies(), delete_policy_makes_get_return_404(), disable_policy_updates_document_but_get_still_works(), disable_policy_with_malformed_json_returns_api_error() (+11 more)

### Community 453 - "tests.rs"
Cohesion: 0.23
Nodes (13): create_path_accepts_family_policies(), family_policy_json_validates_through_endpoint_path(), family_policy_yaml_validates_through_endpoint_path(), invalid_family_policy_returns_structured_issues_and_id(), load_str_and_validate_endpoint_agree_on_valid_yaml(), malformed_yaml_returns_validation_issue(), HeaderMap, unknown_family_is_invalid_with_truncated_echo() (+5 more)

### Community 454 - "RunnerPlanRequest"
Cohesion: 0.15
Nodes (13): type, RunnerPlanRequest, agentDisplayName, systemPrompt, workflowPresent, additionalProperties, description, properties (+5 more)

### Community 455 - "$ref"
Cohesion: 0.15
Nodes (13): description, items, type, default, items, type, $ref, default (+5 more)

### Community 457 - "SourceLabelPolicy"
Cohesion: 0.21
Nodes (10): Confidentiality, Integrity, Option, Origin, Trust, Vec, SourceLabelPolicy, SourceLabelPolicyEntry (+2 more)

### Community 458 - "MemoryBudgetAlertStore"
Cohesion: 0.18
Nodes (15): config(), config_round_trip_and_name_conflict(), firing(), firing_dedup_is_per_config_principal_window(), MemoryBudgetAlertStore, BudgetAlertConfig, BudgetAlertFiring, CreateBudgetAlertConfigRequest (+7 more)

### Community 459 - "retry_integration.rs"
Cohesion: 0.36
Nodes (11): does_not_retry_401(), event(), fast_retry(), gives_up_after_max_attempts(), honors_retry_after_header(), ok_decision_body(), retries_503_until_success(), GuardEvent (+3 more)

### Community 460 - "view_from_record"
Cohesion: 0.27
Nodes (9): NewViewRecord, AnalyticsDashboardView, DateTime, Result, String, Utc, Value, view_from_record() (+1 more)

### Community 461 - "llm_usage.rs"
Cohesion: 0.21
Nodes (17): list_llm_usage(), llm_usage_error_response(), LlmUsageFilter, LlmUsageGroupBy, LlmUsageState, parse_rfc3339(), read_query(), Arc (+9 more)

### Community 463 - "check_and_maybe_regenerate"
Cohesion: 0.21
Nodes (11): append_assistant_turn(), check_and_maybe_regenerate(), Client, Decision, GatewayProviderConnection, Option, P, ResolvedGatewayRoute (+3 more)

### Community 464 - "MemoryHumanReviewStore"
Cohesion: 0.18
Nodes (14): empty_analytics(), key(), MemoryHumanReviewStore, CreateHumanReviewEventRequest, HashMap, HumanReviewAnalyticsFilter, HumanReviewAnalyticsResponse, HumanReviewEvent (+6 more)

### Community 465 - "mod.rs"
Cohesion: 0.36
Nodes (8): authority_template_substitutes_all_placeholders(), build(), hallucination_template_substitutes_all_placeholders(), String, schema(), schemas_have_required_fields(), semantic_policy_template_substitutes_all_placeholders(), tone_template_substitutes_all_placeholders()

### Community 466 - ".analytics"
Cohesion: 0.12
Nodes (27): count_outcome(), group_row(), GroupAccumulator, is_human_intervention(), payload_string(), percentage(), policy_ids(), PolicyAccumulator (+19 more)

### Community 468 - "budget.rs"
Cohesion: 0.22
Nodes (16): window_starts(), admit_llm_budget(), budget_exceeded_response(), evaluate_llm_budget_alerts(), llm_budget_policy_matches(), meter_llm_usage(), monday_is_its_own_week_start(), month_rollover_resets_day_and_month_but_not_week() (+8 more)

### Community 469 - "Plugin contract"
Cohesion: 0.17
Nodes (11): Adding a new language binding, `Context` — anything the customer wants logged but not evaluated, `Decision` — what TrustLoopGuard returns, `Draft` — what the agent wants to do, Plugin contract, Pseudocode, Required behaviors per host adapter, Required behaviors per language binding (+3 more)

### Community 470 - "RunnerReport"
Cohesion: 0.13
Nodes (15): RunnerReport, default, type, error, sessions, status, additionalProperties, description (+7 more)

### Community 471 - "Policy Cookbook"
Cohesion: 0.17
Nodes (12): Apply A Rule To One Agent, Apply A Rule To Voice Only, Auto-Generate Guardrails From An Agent Prompt, Block PII Leakage, CLI, Deletion, Escalate Legal Advice, HTTP (+4 more)

### Community 472 - "route.ts"
Cohesion: 0.14
Nodes (14): attackVectorSchema, dispatchBodySchema, documentTemplateSchema, isBase64(), POST(), MockRustApiError, MockWorkspaceAccessError, proxyMock (+6 more)

### Community 473 - "MemoryToolMetadataStore"
Cohesion: 0.18
Nodes (10): MemoryToolMetadataStore, HashMap, Option, Result, RwLock, Self, String, ToolMetadata (+2 more)

### Community 475 - "ConnectAgentStep.tsx"
Cohesion: 0.08
Nodes (32): ConnectAgentStep(), FirstEventStatus(), FLOW_BEATS, NEXT_STEPS, onboardingContextQuery(), CREATED, CopyBlock(), useFirstTrace() (+24 more)

### Community 476 - "Any"
Cohesion: 0.14
Nodes (19): FactsT, InputT, _build_financial_operation_request(), _clean_financial_operation_field(), _merge_context(), Any, CounterpartyRef, Decision (+11 more)

### Community 477 - "index.mdx"
Cohesion: 0.33
Nodes (5): Run The Chat Demo, Start The Server, Try Gateway Mode, Try The Demo Surfaces, Write Your First Policy

### Community 478 - "BudgetAlertStoreError"
Cohesion: 0.20
Nodes (14): BudgetAlertStoreError, budget_alert_store_error(), config_from_stored(), conflict_aware_error(), firing_from_stored(), PostgresBudgetAlertAdapter, Arc, BudgetAlertConfig (+6 more)

### Community 479 - "PostgresToolMetadataAdapter"
Cohesion: 0.20
Nodes (9): PostgresToolMetadataAdapter, Arc, Option, Result, Self, ToolMetadata, ToolMetadataEntry, Vec (+1 more)

### Community 480 - "Architecture"
Cohesion: 0.18
Nodes (11): Architecture, Customer integration paths, Dashboard-owned surfaces, End-state to keep in mind, Event-centered check model, Latency budget (committed), Request lifecycle (HTTP path), Runtime data flow (+3 more)

### Community 481 - "Team & invites"
Cohesion: 0.18
Nodes (11): Acceptance flow, Authorization model, Endpoints, Enforcement, Invite lifecycle, Memory mode, Ownership, Roles (+3 more)

### Community 483 - "SettingsStore"
Cohesion: 0.10
Nodes (23): WorkspaceApiKeyVerifyError, ApiKeyStore, NewApiKey, Arc, Option, Send, String, Sync (+15 more)

### Community 484 - "core.ts"
Cohesion: 0.10
Nodes (23): testHeldActionDoesNotExecute(), testOfflineAgentApprovesAndExecutesProposedRefund(), testOrderSearch(), testOverRefundStillSubmitsFinancialAction(), testPrepareRefundBuildsTypedAction(), buildRefundActionRequest(), executeRefundTool(), messageForStatus() (+15 more)

### Community 485 - "compilerOptions"
Cohesion: 0.20
Nodes (9): compilerOptions, declaration, lib, outDir, rootDir, types, exclude, extends (+1 more)

### Community 486 - "parse_body"
Cohesion: 0.17
Nodes (13): api_error_response(), ApiErrorCode, Response, StatusCode, String, is_yaml_content_type(), parse_body(), AgentProfile (+5 more)

### Community 487 - "events_integration.rs"
Cohesion: 0.38
Nodes (9): observe_only_decision(), one_shot_retry(), GuardEvent, RetryConfig, Value, run_scoped_client_attaches_run_and_event_ids(), send_email_event(), submit_event_maps_server_error() (+1 more)

### Community 489 - "fresh_repo"
Cohesion: 0.30
Nodes (14): disabled_row_still_readable_with_flag(), fresh_repo(), get_is_isolated_by_workspace(), insert_and_get_round_trips_typed_metadata(), list_returns_only_active_workspace_rows(), negative_cache_serves_repeated_misses(), ContainerAsync, PostgresImage (+6 more)

### Community 490 - "agents.rs"
Cohesion: 0.32
Nodes (12): AgentState, delete_agent(), get_agent(), list_agents(), Arc, Bytes, HeaderMap, Option (+4 more)

### Community 491 - "fresh_repo"
Cohesion: 0.27
Nodes (10): api_key_principal_round_trips_create_list_verify(), batch_revoke_api_keys_is_workspace_scoped(), batch_revoke_api_keys_updates_status_and_auth_lookup(), checker_mode_check_constraint_rejects_invalid_values(), fresh_repo(), get_settings_round_trips_checker_enforcement_modes(), ContainerAsync, DashboardAdminRepo (+2 more)

### Community 492 - "PostgresLlmPricingAdapter"
Cohesion: 0.22
Nodes (7): PostgresLlmPricingAdapter, Arc, Option, Result, Self, Vec, store_error()

### Community 493 - "Red-Team Dispatch"
Cohesion: 0.20
Nodes (10): API, Configuration, Hardening loop, Job lifecycle, Ownership boundary, Red-Team Dispatch, Request flow, Runner contract (+2 more)

### Community 494 - "@trustloopguard/sdk"
Cohesion: 0.20
Nodes (9): Custom handlers, Gateway mode, Guard modes, Installation, License, Low-level client, Quick start, Requirements (+1 more)

### Community 495 - "layout.tsx"
Cohesion: 0.17
Nodes (9): ibmPlexMono, inter, metadata, RootLayoutProps, ThemeProvider(), Toaster(), shouldNotify(), VersionWatcher() (+1 more)

### Community 496 - "scripts"
Cohesion: 0.22
Nodes (9): scripts, build, db:seed, dev, start, test, test:coverage, test:watch (+1 more)

### Community 497 - "SDK publishing"
Cohesion: 0.22
Nodes (6): Before tagging, Common failures, Publish, Release contract, SDK publishing, Verify

### Community 499 - "create_my_workspace"
Cohesion: 0.14
Nodes (27): create_invite(), create_my_workspace(), list_invites(), list_members(), list_my_workspaces(), revoke_invite(), Extension, HeaderMap (+19 more)

### Community 500 - "api_error_response"
Cohesion: 0.24
Nodes (10): ai_edit_policy(), Bytes, Response, api_error_response(), api_error_response_with_details(), ApiErrorCode, Response, StatusCode (+2 more)

### Community 501 - "wire.rs"
Cohesion: 0.29
Nodes (12): call_chat_completions(), malformed_inner_json_yields_parse_error(), missing_content_yields_missing_field(), missing_usage_defaults_to_zero(), parse_chat_response(), parses_well_formed_response(), RequestParts, Client (+4 more)

### Community 502 - "MockRefundClient"
Cohesion: 0.27
Nodes (3): MockRefundClient, timestamp(), FinancialMandateListResponse

### Community 503 - ".submit_event"
Cohesion: 0.31
Nodes (6): Client, Decision, GuardEvent, Option, Result, SdkError

### Community 504 - "header_value"
Cohesion: 0.25
Nodes (8): header_value(), log_http_response(), HeaderMap, Next, Option, Request, Response, String

### Community 505 - "marketing-event-link.tsx"
Cohesion: 0.21
Nodes (13): Footer(), getFooterEvent(), LINK_GROUPS, Status, MarketingEventLink(), MarketingEventLinkProps, mergeRel(), Status (+5 more)

### Community 506 - "JsonSchema"
Cohesion: 0.12
Nodes (20): Duration, Result, JsonSchema, LlmError, LlmOutput, Duration, String, Value (+12 more)

### Community 507 - "MemoryPolicyStore"
Cohesion: 0.36
Nodes (7): MemoryPolicyRecord, MemoryPolicyStore, Arc, HashMap, RwLock, Self, String

### Community 508 - "Authorization"
Cohesion: 0.22
Nodes (9): Authorization, OAuth users (Google / GitHub), See also, Three lanes, one middleware, `TL_API_KEY` — internal / web-to-Rust, User-session JWT — HS256, minted by Rust, What this model does *not* have, Why this shape (+1 more)

### Community 509 - "Gateway"
Cohesion: 0.20
Nodes (10): Budgets + Metering Quickstart, Configuration, Enforcement Response Signal, Gateway, Observability, Ownership, Provider Support, Retention (+2 more)

### Community 510 - "RunnerPlanResponse"
Cohesion: 0.22
Nodes (9): RunnerPlanResponse, vectors, additionalProperties, description, properties, required, type, items (+1 more)

### Community 511 - "feature_request.md"
Cohesion: 0.22
Nodes (8): Acceptance criteria, Additional context, Alternatives considered, Compatibility and migration, Problem, Proposed behavior, SDK/API surface, Summary

### Community 512 - "route.ts"
Cohesion: 0.16
Nodes (11): cleanupAgent(), createAgentSchema, GET(), POST(), stringListSchema, AgentClient, AgentProfileWire, mockState (+3 more)

### Community 513 - "PolicyEditorDialog.test.tsx"
Cohesion: 0.25
Nodes (6): generatePolicyDraft, getPolicy, NON_ROUNDTRIP_YAML, ROUNDTRIP_YAML, upsertPolicy, validatePolicy

### Community 515 - "AuthUserState"
Cohesion: 0.17
Nodes (12): AuthUserState, normalize_oauth_provider(), oauth_session(), Json, Response, Arc, Option, Result (+4 more)

### Community 516 - "ApiError"
Cohesion: 0.15
Nodes (10): ApiError, ApiErrorCode, ApiErrorCode, Display, Formatter, Result, Self, String (+2 more)

### Community 517 - "HumanReviewStoreError"
Cohesion: 0.18
Nodes (18): create_review_event(), human_review_analytics(), list_review_events(), CreateHumanReviewEventRequest, HeaderMap, Json, Path, Response (+10 more)

### Community 518 - "runs_integration.rs"
Cohesion: 0.46
Nodes (7): event_body(), one_shot_retry(), RetryConfig, Value, run_body(), run_helpers_encode_ids_and_parse_typed_responses(), start_run_posts_typed_request_with_bearer_auth()

### Community 519 - "ProvenanceMap"
Cohesion: 0.36
Nodes (5): ProvenanceMap, BTreeMap, Into, String, Vec

### Community 520 - "fresh_repo"
Cohesion: 0.39
Nodes (7): fresh_repo(), insert_then_mark_failed(), insert_then_mark_sent(), list_stale_returns_only_old_pending(), record_attempt_increments_counter(), ContainerAsync, PostgresImage

### Community 521 - "LlmUsageBucketsResponse.ts"
Cohesion: 0.24
Nodes (6): RFC-3339, LlmUsageBucketsResponse, LlmUsageEvent, RFC-3339, LlmUsageListResponse, LlmUsageResponse

### Community 522 - "FinancialMandate"
Cohesion: 0.48
Nodes (3): CreateFinancialMandateRequest, FinancialMandate, FinancialMandate

### Community 523 - "LiveKitSupportAgent"
Cohesion: 0.29
Nodes (3): LiveKitSupportAgent, Decision, Smallest possible LiveKit-style TrustLoopGuard integration.  This is the shape w

### Community 525 - "LiveKit agent guardrail demo"
Cohesion: 0.25
Nodes (7): Files, LiveKit agent guardrail demo, Modes, Run gateway mode, Run SDK mode, SDK mode, Setup (isolated env)

### Community 526 - "Agent-hardening loop"
Cohesion: 0.25
Nodes (8): Agent-hardening loop, Attack-vector planner (`redteam:plan`), Ownership, Saved plans (per-agent library), Seeds reach the attacker, not generic templates, The loop, The workflow graph is the provenance graph, Two honest policy sources

### Community 527 - "Red-Team Report Sharing"
Cohesion: 0.25
Nodes (8): API, Configuration, Red-Team Report Sharing, Rendering, Share tokens, Storage, The report payload, Two surfaces

### Community 528 - "RunnerHandle"
Cohesion: 0.25
Nodes (8): RunnerHandle, type, jobId, additionalProperties, description, properties, required, type

### Community 529 - "Red-Team Runner Contract v1"
Cohesion: 0.25
Nodes (7): Event Fields, `GET /health`, `GET /redteam/jobs/{jobId}`, `POST /redteam/jobs`, Red-Team Runner Contract v1, Session Fields, Transport

### Community 530 - "Integration & Interception — How TrustLoopGuard Hooks an Agent"
Cohesion: 0.25
Nodes (7): Bottom line, Concrete trace (email agent), Integration & Interception — How TrustLoopGuard Hooks an Agent, Integration tiers, The framework's role (LiveKit example), The key truth: the LLM never runs anything. It only *asks.*, Where TrustLoopGuard intercepts

### Community 531 - "compilerOptions"
Cohesion: 0.25
Nodes (7): compilerOptions, declaration, lib, outDir, rootDir, extends, include

### Community 532 - "guard-modes.mdx"
Cohesion: 0.29
Nodes (6): Choosing A Mode, Modes, Rewrite, Rewrite Or Regenerate, Streaming output, Strict

### Community 533 - "properties"
Cohesion: 0.07
Nodes (30): $ref, description, items, type, default, $ref, anyOf, description (+22 more)

### Community 534 - "properties"
Cohesion: 0.20
Nodes (10): anyOf, properties, approval, reversible, sandbox_hint, side_effect, tool, type (+2 more)

### Community 535 - "policy_ast.rs"
Cohesion: 0.20
Nodes (10): Action, default_severity(), MatchClause, Matcher, Channel, Matcher, Severity, String (+2 more)

### Community 536 - "LabelBasisSet"
Cohesion: 0.11
Nodes (20): allOf, default, $ref, LabelBasisSet, Labels, allOf, default, $ref (+12 more)

### Community 537 - "properties"
Cohesion: 0.17
Nodes (12): properties, required, type, items, type, ApprovalRule, type, approver_roles (+4 more)

### Community 539 - ".generate_guardrails"
Cohesion: 0.38
Nodes (5): Client, GuardrailGenerateResponse, GuardrailListResponse, Result, SdkError

### Community 540 - "api_error_response"
Cohesion: 0.43
Nodes (6): api_error_response(), log_api_error(), ApiErrorCode, Response, StatusCode, String

### Community 542 - "HandlerCtx"
Cohesion: 0.05
Nodes (56): FuzzyChecker, FuzzyHit, HandlerCtx, NoOpFuzzyChecker, NoOpProfileResolver, ProfileResolver, Action, AgentProfile (+48 more)

### Community 543 - "properties"
Cohesion: 0.20
Nodes (10): properties, required, type, AllowedSource, type, $ref, kind, origin (+2 more)

### Community 544 - "definitions"
Cohesion: 0.14
Nodes (14): definitions, LimitAction, Origin, ParamRole, SideEffectClass, description, enum, type (+6 more)

### Community 545 - "source-label-policy.schema.json"
Cohesion: 0.07
Nodes (27): anyOf, enum, type, definitions, Confidentiality, Integrity, Origin, Trust (+19 more)

### Community 546 - "params"
Cohesion: 0.25
Nodes (8): items, type, $ref, default, items, type, allowed_sources, params

### Community 547 - "properties"
Cohesion: 0.18
Nodes (11): ParamSpec, anyOf, description, properties, required, type, type, limit (+3 more)

### Community 548 - "Analytics Dashboards"
Cohesion: 0.29
Nodes (7): Access, Analytics Dashboards, Ownership, Queries, Saved Views, Template Variables, Widget Layout

### Community 549 - "Embedder"
Cohesion: 0.22
Nodes (6): Embedder, MockEmbedder, Default, Self, Send, Sync

### Community 550 - "devDependencies"
Cohesion: 0.29
Nodes (7): devDependencies, lefthook, prettier, secretlint, @secretlint/secretlint-rule-preset-recommend, tsx, yaml

### Community 556 - "RunnerAttackSession"
Cohesion: 0.40
Nodes (5): RunnerAttackSession, additionalProperties, description, required, type

### Community 557 - "source_chain"
Cohesion: 0.50
Nodes (4): type, source_chain, items, type

### Community 558 - "properties"
Cohesion: 0.14
Nodes (14): items, type, ParamSpec, anyOf, description, properties, required, type (+6 more)

### Community 560 - "proxy_anthropic_messages"
Cohesion: 0.38
Nodes (9): proxy_anthropic_messages(), proxy_openai_chat_completions(), Bytes, Extension, HeaderMap, Option, Path, Response (+1 more)

### Community 561 - ".list_policies"
Cohesion: 0.31
Nodes (7): Client, Option, PolicyDocument, PolicyFamily, PolicyListResponse, Result, SdkError

### Community 562 - "docs"
Cohesion: 0.33
Nodes (5): Content, Develop, docs, Password protection, Why a separate app

### Community 565 - "package.json"
Cohesion: 0.33
Nodes (5): license, name, private, type, version

### Community 567 - "service.rs"
Cohesion: 0.09
Nodes (47): financial_matches(), action_decision(), authorization_scope_summary(), compose_policy_decisions(), decision_receipt_reason(), evidence_bool(), evidence_for_key(), financial_approver_roles() (+39 more)

### Community 569 - "WorkflowDefinition"
Cohesion: 0.20
Nodes (10): description, WorkflowDefinition, definition, source, description, type, description, properties (+2 more)

### Community 570 - "LabelResolution"
Cohesion: 0.13
Nodes (15): $ref, LabelResolution, additionalProperties, description, type, description, properties, required (+7 more)

### Community 571 - "validate_create_action"
Cohesion: 0.33
Nodes (8): clean_operation(), clean_required(), is_valid_transition(), CreateFinancialActionRequest, FinancialActionStatus, Result, String, validate_create_action()

### Community 572 - "CheckerRun"
Cohesion: 0.15
Nodes (13): type, description, properties, required, type, CheckerRun, items, type (+5 more)

### Community 573 - "query_parts"
Cohesion: 0.33
Nodes (8): query_parts(), read_filter(), read_limit(), HumanReviewAnalyticsFilter, Item, Iterator, Option, String

### Community 575 - "Financial Authorization"
Cohesion: 0.25
Nodes (8): Contract, Durable Storage, Evidence And Eligibility, Financial Authorization, HTTP API, Outcome Data, Policy Family, Reversal Semantics

### Community 577 - "Financial Authorization Contract TDD Evidence"
Cohesion: 0.29
Nodes (6): Completion Status, Financial Authorization Contract TDD Evidence, RED/GREEN Evidence, Test Specification, User Journeys, Validation Commands

### Community 578 - "MemoryKnowledgeStore"
Cohesion: 0.22
Nodes (9): MemoryKnowledgeStore, HashMap, KnowledgeSourceDocument, KnowledgeSourceFileResponse, Result, RwLock, Self, String (+1 more)

### Community 579 - "proxy.ts"
Cohesion: 0.43
Nodes (7): config, isAuthenticated(), isPublicPath(), proxy(), PUBLIC_PATH_PREFIXES, safeRedirect(), SESSION_COOKIE_NAMES

### Community 580 - "auth.ts"
Cohesion: 0.08
Nodes (34): POST(), AuthScreen(), OrDivider(), CredentialsForm(), safeRedirect(), SignInPage(), safeRedirect(), SignUpPage() (+26 more)

### Community 581 - "Environments"
Cohesion: 0.33
Nodes (6): API, Environments, Ownership, Policy Deployment, Relationship to Workspaces, Runtime Resolution

### Community 582 - "TrustLoopGuard concepts"
Cohesion: 0.33
Nodes (6): Diagram workflow, Reading order, TrustLoopGuard concepts, Visual map, What TrustLoopGuard is, When to update these docs

### Community 583 - "Runs"
Cohesion: 0.33
Nodes (6): Events, External ID, Lifecycle, Ownership, Relationship to traces, Runs

### Community 584 - "Merge gates"
Cohesion: 0.33
Nodes (5): Bypass policy, Enabling required status checks (one-time, GitHub Settings), Merge gates, Updating the gate set, What the gates *don't* enforce

### Community 586 - "budget_alert.rs"
Cohesion: 0.33
Nodes (12): BudgetAlertConfig, BudgetAlertConfigListResponse, BudgetAlertFiring, BudgetAlertFiringListResponse, BudgetAlertThresholdType, BudgetAlertWindow, CreateBudgetAlertConfigRequest, Option (+4 more)

### Community 587 - "checks.rs"
Cohesion: 0.33
Nodes (9): check_gateway_content(), GatewayContentCheck, GatewayDecisionLog, log_gateway_decision(), Decision, Option, ResolvedGatewayRoute, Response (+1 more)

### Community 588 - "dashboard-widgets.tsx"
Cohesion: 0.07
Nodes (30): Verdict, VerdictLegend(), VerdictLegendProps, VERDICTS, BadgeVariant, countVerdicts(), DASHBOARD_WIDGET_KEYS, DASHBOARD_WIDGETS (+22 more)

### Community 589 - "SignalEvidence"
Cohesion: 0.18
Nodes (11): SignalEvidence, type, message, provider_id, severity, type, anyOf, description (+3 more)

### Community 590 - "Policies"
Cohesion: 0.40
Nodes (5): API, Environment Enablement, Policies, Registry, Runtime Boundaries

### Community 592 - "hallucination.md"
Cohesion: 0.40
Nodes (4): Agent profile, Conversation, Grounding documents, Task

### Community 593 - "semantic_policy.md"
Cohesion: 0.40
Nodes (4): Event, Instructions, Policy, Proposed output

### Community 594 - "any_policy_summary"
Cohesion: 0.29
Nodes (9): any_policy_summary(), normalize_policy_ids(), policy_action(), policy_summary(), Action, PolicySummary, Result, String (+1 more)

### Community 595 - "insert_trace"
Cohesion: 0.29
Nodes (10): analytics_distinguishes_guardrail_and_human_interventions(), fresh_pool(), insert_trace(), review_events_are_append_only_and_latest_is_queryable(), ContainerAsync, DbPool, Option, PostgresImage (+2 more)

### Community 596 - "The three rules"
Cohesion: 0.50
Nodes (4): 1. Engine-only PRs aren't done, 2. No internal imports in `demo/`, 3. Cross-cutting concerns live in the SDK, once, The three rules

### Community 597 - "route.ts"
Cohesion: 0.60
Nodes (4): forwardToWebhook(), hits, isRateLimited(), POST()

### Community 598 - "llm_usage.rs"
Cohesion: 0.33
Nodes (9): LlmUsageBucket, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageListResponse, LlmUsageResponse, Option, String, Value (+1 more)

### Community 599 - "Web Dashboard And Authentication"
Cohesion: 0.40
Nodes (5): Acceptance Criteria, Authentication, Dashboard Data Boundary, Status, Web Dashboard And Authentication

### Community 600 - "tool-metadata.schema.json"
Cohesion: 0.40
Nodes (4): required, $schema, title, type

### Community 601 - "seo.ts"
Cohesion: 0.14
Nodes (19): inter, metadata, RootLayout(), RootLayoutProps, spaceGrotesk, robots(), HOME_LAST_MODIFIED, sitemap() (+11 more)

### Community 603 - "authority.md"
Cohesion: 0.50
Nodes (3): Agent authority profile, Conversation, Task

### Community 604 - "tone.md"
Cohesion: 0.50
Nodes (3): Agent tone profile, Conversation, Task

### Community 607 - "Human Review Analytics"
Cohesion: 0.50
Nodes (4): Analytics, Human Review Analytics, Ownership, Review Events

### Community 608 - "generate-openapi-docs.mjs"
Cohesion: 0.40
Nodes (3): generatedPages, meta, openapi

### Community 612 - "OpenAiClient"
Cohesion: 0.32
Nodes (7): OpenAiClient, Client, Duration, Into, Result, Self, String

### Community 615 - "auth.rs"
Cohesion: 0.48
Nodes (6): AuthRequest, AuthResponse, ChangePasswordRequest, OAuthIdentityRequest, Option, String

### Community 618 - "KnowledgeStoreError"
Cohesion: 0.38
Nodes (10): KnowledgeStoreError, CreateKnowledgeSourceRequest, String, decode_file_data(), CreateKnowledgeSourceRequest, Result, Vec, validate_create_request() (+2 more)

### Community 619 - "AnalyticsFact"
Cohesion: 0.35
Nodes (10): AnalyticsFact, AnalyticsRepo, payload_string(), policy_ids(), Option, Result, String, Value (+2 more)

### Community 622 - "generate_guardrails"
Cohesion: 0.13
Nodes (26): draft_policy(), Bytes, Response, parse_policy_set(), policy_draft_item_schema(), policy_draft_json_schema(), policy_from_draft(), policy_set_draft_json_schema() (+18 more)

### Community 669 - "proxy_provider_request"
Cohesion: 0.16
Nodes (18): proxy_provider_request(), Bytes, GatewayProviderKind, HeaderMap, Option, P, Response, String (+10 more)

### Community 670 - "RecordingTraceStore"
Cohesion: 0.31
Nodes (7): RecordingTraceStore, Mutex, Option, Result, String, TraceSummary, Vec

### Community 671 - "properties"
Cohesion: 0.15
Nodes (13): KnowledgeSource, type, type, allOf, default, properties, required, type (+5 more)

### Community 673 - "proxy_healthcare_agent.py"
Cohesion: 0.27
Nodes (8): entrypoint(), gateway_api_key(), gateway_openai_base_url(), HealthcareProxyAgent, livekit_run_external_id(), Agent, JobContext, LiveKit healthcare agent that routes its LLM through TrustLoopGuard gateway.  Th

### Community 674 - "validate_create_event"
Cohesion: 0.25
Nodes (8): clean_string(), normalize_metadata(), CreateHumanReviewEventRequest, Option, Result, String, Value, validate_create_event()

### Community 676 - "fresh_pool"
Cohesion: 0.38
Nodes (6): event(), fresh_pool(), insert_window_sum_and_grouping_round_trip(), ContainerAsync, DbPool, PostgresImage

### Community 677 - "prepare_streaming_request"
Cohesion: 0.36
Nodes (8): parse_provider_request(), prepare_streaming_request(), Bytes, EnforcementProfile, P, Response, Result, Value

### Community 915 - "gateway.mdx"
Cohesion: 0.20
Nodes (9): Anthropic clients, Configuration model, Current limits, Enforcement signals, OpenAI-compatible clients, Quick start, Streaming, Verify the connection (+1 more)

### Community 1138 - "LlmModelPrice.ts"
Cohesion: 0.47
Nodes (3): LlmModelPrice, LlmPriceSource, LlmPricingListResponse

### Community 1581 - "fresh_repos"
Cohesion: 0.33
Nodes (6): create_workspace_seeds_enabled_starter_policies(), fresh_repos(), ContainerAsync, PolicyRepo, PostgresImage, TeamRepo

### Community 1774 - "setup.ts"
Cohesion: 0.13
Nodes (22): guardedPayout(), headers(), main(), registerTool(), AGENT_DEMO_WORLD_PORT, createClient(), demoRoot(), fetchWithWorkspace() (+14 more)

### Community 1803 - "defaults.rs"
Cohesion: 0.33
Nodes (5): default_views(), empty_catalog(), AnalyticsDashboardView, AnalyticsFacetCatalogResponse, Vec

### Community 1805 - "fresh_pool"
Cohesion: 0.40
Nodes (5): fresh_pool(), ContainerAsync, DbPool, PostgresImage, upsert_get_list_and_delete_round_trip()

### Community 1807 - "Red-team harden (policy synthesis)"
Cohesion: 0.29
Nodes (7): Inputs and outputs, Outcome model, Ownership, Reachable substrates, Red-team harden (policy synthesis), What it does, Where it sits

### Community 1809 - "index.mdx"
Cohesion: 0.40
Nodes (4): Core Ideas, Latency Model, Runtime Shape, Source Of Truth

### Community 1811 - "HumanReviewAnalyticsFilter"
Cohesion: 0.50
Nodes (3): HumanReviewAnalyticsFilter, Option, String

### Community 1812 - "setup.ts"
Cohesion: 0.70
Nodes (4): enforceModes(), headers(), main(), tools

### Community 1813 - "Agent Breakaway Arena"
Cohesion: 0.33
Nodes (6): Adapter Contract, Agent Breakaway Arena, Flow, Hardening Loop, Ownership Boundary, What The Agent Receives

### Community 1814 - "llm_pricing.rs"
Cohesion: 0.38
Nodes (6): LlmModelPrice, LlmPriceSource, LlmPricingListResponse, String, Vec, UpsertLlmModelPriceRequest

### Community 1816 - "Verdict"
Cohesion: 0.50
Nodes (4): Verdict, description, enum, type

### Community 1817 - "submit_event"
Cohesion: 0.36
Nodes (7): GuardEvent, HeaderMap, Json, Response, String, submit_event(), workspace_id_for_event()

### Community 1820 - "index.mdx"
Cohesion: 0.40
Nodes (4): CLI, HTTP API, Rust Crates, SDKs

### Community 1821 - ".__init__"
Cohesion: 0.40
Nodes (3): AsyncBaseTransport, BaseTransport, RetryConfig

### Community 1824 - "hash_password"
Cohesion: 0.39
Nodes (7): hash_password(), PasswordError, Result, String, verify_password(), hash_roundtrip_matches(), verify_rejects_wrong_password()

### Community 1826 - "api_error"
Cohesion: 0.39
Nodes (7): api_error(), invalid_credentials(), password_auth_disabled(), ApiErrorCode, Response, StatusCode, String

### Community 1827 - "gateway_routes"
Cohesion: 0.50
Nodes (4): build_gateway_http_client(), gateway_routes(), Client, Router

### Community 1830 - "BudgetAlertFiring"
Cohesion: 0.50
Nodes (3): BudgetAlertFiring, RFC-3339, BudgetAlertFiringListResponse

### Community 1831 - "latency_ms"
Cohesion: 0.50
Nodes (4): format, minimum, type, latency_ms

### Community 1832 - "LlmPricingStore"
Cohesion: 0.67
Nodes (3): LlmPricingStore, Send, Sync

## Knowledge Gaps
- **2811 isolated node(s):** `printWidth`, `tabWidth`, `useTabs`, `semi`, `singleQuote` (+2806 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **648 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `StorageError` connect `StorageError` to `Result`, `BudgetAlertRepo`, `PostgresRedteamJobAdapter`, `HumanReviewAnalyticsFilter`, `EnvironmentRepo`, `tests.rs`, `RedteamPlanRepo`, `PostgresUserAdapter`, `api_keys.rs`, `Result`, `event_summary`, `latest_review_outcomes`, `PostgresGatewayAdapter`, `insert_existing_workspace_member`, `PostgresHumanReviewAdapter`, `ToolMetadataRepo`, `latest_review_outcomes`, `PostgresFinancialAdapter`, `.create_event`, `validation.rs`, `LlmUsageRepo`, `RunRepo`, `environment_deployments.rs`, `LlmUsageStoreError`, `LlmPricingRepo`, `PostgresLabelPolicyAdapter`, `profile_record_to_wire`, `UserRepo`, `EnvironmentStoreError`, `lib.rs`, `PolicyRepo`, `RunStoreError`, `view_from_record`, `models.rs`, `.analytics`, `AgentRepo`, `.create_event`, `.query`, `BudgetAlertStoreError`, `PostgresToolMetadataAdapter`, `schema.rs`, `SettingsStore`, `PostgresAnalyticsAdapter`, `share.rs`, `metrics.rs`, `EscalationRepo`, `AnalyticsFact`, `PostgresLlmPricingAdapter`, `writer.rs`, `plan.rs`, `RedteamReportShareRepo`, `TeamStoreError`, `KnowledgeRepo`, `dashboard_admin_repo.rs`, `run_summary`?**
  _High betweenness centrality (0.097) - this node is a cross-community bridge._
- **Why does `AppState` connect `AppState` to `oauth.rs`, `traces.rs`, `router`, `submit_event`, `proxy_provider_request`, `HandlerCtx`, `gateway_routes`, `LlmPricingStore`, `event_service.rs`, `build_app_state`, `checks.rs`, `check_and_maybe_regenerate`, `authorize_workspace_admin`, `JwtSigner`, `mod.rs`, `budget.rs`, `build_postgres_layer`, `budget_alerts.rs`, `financial_actions.rs`, `RedteamJobStore`, `GatewayState`, `WorkspaceKeyContext`, `SettingsStore`, `effective_checker_modes`, `share.rs`, `PolicyStore`, `event_ingestion.rs`, `engine.rs`, `escalation.rs`?**
  _High betweenness centrality (0.072) - this node is a cross-community bridge._
- **Why does `State` connect `State` to `AuthUserState`, `HumanReviewStoreError`, `put_llm_price`, `oauth.rs`, `harden_job`, `api_error_response`, `traces.rs`, `submit_event`, `handlers.rs`, `proxy_anthropic_messages`, `create_knowledge_source`, `ToolMetadataStoreError`, `EnvironmentStoreError`, `PolicyState`, `llm_usage.rs`, `AuthConfig`, `change_password`, `attacks-panel.tsx`, `RedteamState`, `GatewayState`, `WorkspaceKeyContext`, `RunState`, `agents.rs`, `generate_guardrails`, `create_my_workspace`, `create_api_key`, `api_error_response`, `plan.rs`?**
  _High betweenness centrality (0.062) - this node is a cross-community bridge._
- **Are the 90 inferred relationships involving `Client` (e.g. with `Decode` and `SdkError`) actually correct?**
  _`Client` has 90 INFERRED edges - model-reasoned connections that need verification._
- **What connects `printWidth`, `tabWidth`, `useTabs` to the rest of the system?**
  _2886 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Client` be split into smaller, more focused modules?**
  _Cohesion score 0.06376811594202898 - nodes in this community are weakly interconnected._
- **Should `GuardEvent` be split into smaller, more focused modules?**
  _Cohesion score 0.12105263157894737 - nodes in this community are weakly interconnected._