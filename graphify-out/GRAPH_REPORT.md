# Graph Report - TrustLoopGuard  (2026-07-07)

## Corpus Check
- 1299 files · ~635,664 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 13863 nodes · 28243 edges · 1815 communities (1169 shown, 646 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 1575 edges (avg confidence: 0.7)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `ba674bb3`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Client
- GuardEvent
- cn
- AnalyticsCatalogDimension
- fetchMock
- AsyncClient
- oauth.rs
- FinancialActionsContent.tsx
- mod.rs
- PoliciesPageContent.tsx
- Integrating TrustLoopGuard
- code:block1 (POST /v1/check)
- proxyRustJson
- Field-by-field
- code:yaml (id: refund-guarantee)
- plan.rs
- ApiErrorCode
- redteam.rs
- settings_update.rs
- types.py
- PostgresStore
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
- knowledge.rs
- label.rs
- BadgeProps
- auth.rs
- Ownership
- code:block1 (+-------------------+      CheckRequest       +-------------)
- Domain terms
- .create_event
- agent.rs
- tests.rs
- apiKeyHeaders
- scenarios.core.ts
- report.rs
- profile_record_to_wire
- ._run_with_retry
- Gateway Proxy Runtime Branch Guide
- EnvironmentStoreError
- properties
- code:text (policies/refund-promise.yaml)
- GuardEvent.ts
- dashboard-data.ts
- scripts
- generate_guardrails
- rustApiForAuthorizedWorkspace
- policies.ts
- PolicyStoreError
- PolicyState
- models.rs
- RunDetailLiveView.tsx
- synthesis.rs
- properties
- _shared.ts
- index.ts
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
- EventPipelineCtx
- FinancialAuthorizationService
- path
- Policy
- normalization.rs
- event_ingestion.rs
- SdkError
- MemoryAnalyticsStore
- redteam-report.ts
- tests.rs
- req
- payload
- page.tsx
- financial-actions.test.ts
- AnalyticsDashboardWidget.ts
- RedteamReportShareRepo
- PostgresTraceAdapter
- Technical terms
- tool-runner.ts
- code:text (agent drafts risky output)
- dashboard_admin_repo.rs
- MemoryPolicyStore
- financial_authorization_service.rs
- family_parse.rs
- WorkspaceDashboard.tsx
- Result
- client.ts
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
- tlClientForRequest
- code:bash (npm view @trustloopguard/sdk version)
- api_error_response
- package.json
- harden-job-card.tsx
- RedteamJobStoreError
- gateway_budget.rs
- guard.rs
- seo-landing-page.tsx
- SAMPLES
- properties
- Result
- MemoryFinancialStore
- properties
- type
- labels.rs
- Repository Agent Instructions
- latest_review_outcomes
- ToolMetadataRepo
- TierResult
- tier_results
- code:block1 (agent proposes output → trustloop.check(...) → allow | block)
- v0 Design Decisions
- Runtime Refactor Jobs
- agents
- FinancialStoreError
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
- JwtSigner
- drizzle-kit
- db:generate
- pull_request_template.md
- core.ts
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
- load_str
- core.ts
- MemoryRunStore
- code:text (UI component)
- RedteamJobStore
- lint-storage-boundaries.sh
- Decision.ts
- lint-api-contracts.sh
- validation.rs
- dashboard.rs
- policy.rs
- redteam_runner.rs
- check-schema-drift.sh
- AnyPolicy
- value_limit.rs
- metrics.rs
- RunState
- tool_metadata.rs
- EscalationRepo
- RouterConfig
- package.json
- guarded_healthcare_agent.py
- ProfileResolver
- code:text (Browser / SDK)
- AgentProfile.ts
- code:ts (const decision = await client.check({)
- README.md
- code:text (Customer app -> SDK -> /v1/check -> Decision -> customer han)
- workflow_analyzer.rs
- spawn_escalation_worker
- TeamStoreError
- GatewayStoreError
- KnowledgeRepo
- MemoryAnalyticsStore
- writer.rs
- package.json
- LabelPolicyStoreError
- DashboardAdminStoreError
- create_knowledge_source
- .prettierrc.json
- dependencies
- MokaCache
- check.ts
- code:text (app -> /v1/gateway/<route_id>/openai -> TrustLoopGuard -> pr)
- GatewayPageContent.tsx
- event
- tool.rs
- output_safe_response
- label_policy.rs
- code:text (source of truth)
- hero.tsx
- code:text (Dashboard / customer integration)
- properties
- code:bash (npm install @trustloopguard/sdk)
- harden.rs
- gateway.rs
- writer.rs
- memory.rs
- traces.rs
- EnvironmentRepo
- Result
- Contributing to TrustLoopGuard
- RedteamPlanRepo
- Engine
- precommit-typecheck.sh
- definitions
- redteam-core.ts
- precommit-secretlint.sh
- ReviewQueueContent.tsx
- api_keys.rs
- code:py (import trustloopguard as trustloop)
- insert_existing_workspace_member
- aggregate
- State
- human_review.rs
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
- tests.rs
- gateway_repo.rs
- knip.json
- Code of Conduct
- PostgresLabelPolicyAdapter
- RetryConfig
- RunnerAttackVector
- decision.schema.json
- sdk.tsx
- prepush-fast.sh
- LabelPolicyProvider
- RedactedEntity
- parse_retry_after
- Write Your First Policy
- RunStoreError
- render-diagrams.sh
- evaluate_financial_policies
- StorageError
- tests.rs
- GuardEvent Redaction Spec
- embedder.rs
- route.ts
- financial_actions.rs
- create_review_event
- AppState
- financial_actions_integration.rs
- team.rs
- executor.rs
- lib.rs
- analytics.rs
- Channel
- Security Policy
- properties
- RunnerDocumentTemplate
- TrustLoopGuard Hardening v2 — Attack-Grounded Policy Synthesis
- patch_enforcement_profile
- components.json
- policy_repo.rs
- RunSummary
- effective_checker_modes
- severity
- MemoryKnowledgeStore
- LlmClient
- tests.rs
- setup.ts
- test_financial_actions.py
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
- ReportRateLimiter
- guardrails.rs
- financial_repo.rs
- module_exports.rs
- Crates
- redteam-runner.schema.json
- test_events.py
- PostgresAnalyticsAdapter
- TierResult
- docs-auth.ts
- scripts
- code:sh (pnpm --filter @trustloopguard/example-typescript start \)
- human_review.rs
- .from_response
- put_llm_price
- Common Workflows
- main.rs
- env.ts
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
- PolicyError
- compilerOptions
- verify_candidate
- PostgresUserAdapter
- types.ts
- code:py (retry=RetryConfig(max_attempts=1, total_budget_s=0.25))
- OpenAiClient
- fresh_store
- seal_key_material
- properties
- openai-agent.ts
- page.tsx
- build_app
- validation.rs
- check_pipeline.rs
- definitions
- MemoryToolMetadataStore
- Human Review Analytics Spec
- WorkflowRequirement
- HumanReviewAnalyticsResponse.ts
- .submit_event
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
- mod.rs
- request
- lib.rs
- RunnerPlanRequest
- $ref
- 4. Goal-Driven Execution
- SourceLabelPolicy
- AnalyticsStoreError
- retry_integration.rs
- FinancialPolicyRecord.ts
- llm_usage.rs
- page.tsx
- check_and_maybe_regenerate
- MemoryHumanReviewStore
- route.test.ts
- analytics.rs
- 1. Think Before Coding
- budget.rs
- Plugin contract
- RunnerReport
- Policy Cookbook
- route.ts
- redaction
- TeamStoreError
- ConnectAgentStep.tsx
- context.rs
- Embedder
- tests.rs
- route.test.ts
- Architecture
- Team & invites
- 2. Simplicity First
- ui.ts
- compilerOptions
- parse_body
- events_integration.rs
- package.json
- fresh_repo
- gateway.mdx
- Red-Team Dispatch
- @trustloopguard/sdk
- layout.tsx
- scripts
- SDK publishing
- code:bash (curl -X POST $TLG_URL/v1/check \)
- create_my_workspace
- api-keys.ts
- JsonSchema
- proxy_healthcare_agent.py
- .submit_event
- header_value
- marketing-event-link.tsx
- mod.rs
- MemoryPolicyStore
- Authorization
- Gateway
- RunnerPlanResponse
- feature_request.md
- KnowledgeStoreError
- PolicyEditorDialog.test.tsx
- code:json ({)
- auth_user.rs
- ApiError
- patch_gateway_route
- runs_integration.rs
- ProvenanceMap
- fresh_repo
- ToolMetadataProvider
- route.ts
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
- .create_financial_policy
- devDependencies
- code:text (Customer / integrator runtime)
- analytics_query.rs
- Verdict
- code:text (1. [Step] -> verify: [check])
- code:bash (make quickstart)
- RunnerAttackSession
- source_chain
- properties
- code:block2 (CheckRequest)
- GatewayState
- .list_policies
- docs
- KnowledgeSourceDocument.ts
- package.json
- service.rs
- WorkflowDefinition
- LabelResolution
- validate_create_action
- CheckerRun
- query_parts
- Financial Authorization
- Financial Authorization Contract TDD Evidence
- defaults.rs
- proxy.ts
- api_error_response
- Environments
- TrustLoopGuard concepts
- Runs
- Merge gates
- index.mdx
- gateway_routes
- LlmUsageResponse.ts
- SignalEvidence
- Policies
- README.md
- hallucination.md
- semantic_policy.md
- finalize_gateway_response
- insert_trace
- The three rules
- view_from_record
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
- next.config.mjs
- auth.rs
- default_settings
- validate_create_event
- submit_event
- source.config.ts
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
- api_error
- index.mdx
- proxy_provider_request
- monitoring_sessions.rs
- properties
- HumanReviewStoreError
- llm_pricing.rs
- fresh_pool
- auth-redirect.ts
- hash_password
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
- fresh_pool
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
- index.mdx
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
- input_enforcement.rs
- trivial_schema
- Verdict
- Red-team harden (policy synthesis)
- UpsertLlmModelPriceRequest.ts
- Agent Breakaway Arena

## God Nodes (most connected - your core abstractions)
1. `StorageError` - 346 edges
2. `cn()` - 180 edges
3. `Client` - 138 edges
4. `State` - 119 edges
5. `FinancialStoreError` - 112 edges
6. `AsyncClient` - 106 edges
7. `Policy` - 81 edges
8. `AppState` - 78 edges
9. `Client` - 75 edges
10. `path()` - 62 edges

## Surprising Connections (you probably didn't know these)
- `createOutputGuard()` --indirect_call--> `decision()`  [INFERRED]
  sdks/typescript/src/guard.ts → apps/mcp-server/src/handlers.test.ts
- `DecisionHandler` --indirect_call--> `decision()`  [INFERRED]
  sdks/typescript/src/guard.ts → apps/mcp-server/src/handlers.test.ts
- `POST()` --references--> `REQUEST`  [EXTRACTED]
  apps/web/app/api/api-keys/route.ts → sdks/typescript/test/financial-actions.test.ts
- `POST()` --references--> `REQUEST`  [EXTRACTED]
  apps/web/app/api/oauth/authorize/route.ts → sdks/typescript/test/financial-actions.test.ts
- `POST()` --references--> `REQUEST`  [EXTRACTED]
  apps/web/app/api/team/invites/route.ts → sdks/typescript/test/financial-actions.test.ts

## Import Cycles
- 1-file cycle: `crates/tl-server/src/app/openapi.rs -> crates/tl-server/src/app/openapi.rs`
- 1-file cycle: `crates/tl-server/src/state/postgres_adapters/human_review.rs -> crates/tl-server/src/state/postgres_adapters/human_review.rs`
- 1-file cycle: `crates/tl-storage/src/escalations.rs -> crates/tl-storage/src/escalations.rs`
- 1-file cycle: `crates/tl-server/src/state/postgres_adapters/dashboard_admin.rs -> crates/tl-server/src/state/postgres_adapters/dashboard_admin.rs`
- 2-file cycle: `crates/tl-server/src/policies.rs -> crates/tl-server/src/policies/authoring.rs -> crates/tl-server/src/policies.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/llm_pricing_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/writer.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/escalations.rs -> crates/tl-storage/src/lib.rs -> crates/tl-storage/src/escalations.rs`
- 2-file cycle: `crates/tl-storage/src/gateway_repo.rs -> crates/tl-storage/src/lib.rs -> crates/tl-storage/src/gateway_repo.rs`
- 2-file cycle: `crates/tl-storage/src/knowledge_repo.rs -> crates/tl-storage/src/lib.rs -> crates/tl-storage/src/knowledge_repo.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_plan_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/user_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/policy_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_job_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/redteam_report_share_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-storage/src/lib.rs -> crates/tl-storage/src/trace_repo.rs -> crates/tl-storage/src/lib.rs`
- 2-file cycle: `crates/tl-server/src/redteam/mod.rs -> crates/tl-server/src/redteam/share.rs -> crates/tl-server/src/redteam/mod.rs`
- 3-file cycle: `crates/tl-server/src/redteam/handlers.rs -> crates/tl-server/src/redteam/share.rs -> crates/tl-server/src/redteam/mod.rs -> crates/tl-server/src/redteam/handlers.rs`

## Communities (1815 total, 646 thin omitted)

### Community 1 - "Client"
Cohesion: 0.07
Nodes (51): AsyncBaseTransport, BaseTransport, OnAllowSync, OnBlockSync, OnErrorSync, OnEscalateSync, OnReviseSync, Client (+43 more)

### Community 2 - "GuardEvent"
Cohesion: 0.12
Nodes (19): Action, EventKind, GuardEvent, Principal, Action, CheckerRun, EventKind, Option (+11 more)

### Community 3 - "cn"
Cohesion: 0.03
Nodes (131): AgentFilter(), AgentFilterProps, AppSidebar(), AppSidebarProps, data, NavGroup, NavItem, NavMain() (+123 more)

### Community 6 - "AsyncClient"
Cohesion: 0.08
Nodes (72): FactsT, InputT, AsyncClient, AsyncFinancialOperation, _AsyncRunContext, _AsyncRunEventContext, _build_financial_operation_request(), _clean_financial_operation_field() (+64 more)

### Community 9 - "oauth.rs"
Cohesion: 0.08
Nodes (56): caps_per_retry_delay_at_max_delay(), honors_retry_after_when_longer_than_jittered(), ignores_retry_after_when_jitter_already_longer(), invalid(), jitter_fraction_clamps_to_unit_interval(), non_retriable_errors_stop_immediately(), rate_limited(), retries_unavailable_with_exponential_backoff() (+48 more)

### Community 10 - "FinancialActionsContent.tsx"
Cohesion: 0.06
Nodes (57): ChangePasswordCardProps, AuthScreenProps, BrandRailProps, VERDICTS, Card(), CardContent(), CardDescription(), CardFooter() (+49 more)

### Community 11 - "mod.rs"
Cohesion: 0.19
Nodes (33): checker_ctx(), client_submitted_checker_evidence_never_survives(), ctx_with_metadata(), enforce_mode_applies_worst_finding_to_decision(), enforce_mode_with_no_findings_keeps_decision_byte_identical(), event_pipeline_no_op_context_has_all_collaborators(), high_fidelity_event(), modes_gate_each_checker_independently() (+25 more)

### Community 12 - "PoliciesPageContent.tsx"
Cohesion: 0.03
Nodes (138): TTL_OPTIONS, FormError(), CredentialsFormProps, SignupFormProps, AgentOption, FieldProps, FieldRenderIds, PolicyFormProps (+130 more)

### Community 13 - "Integrating TrustLoopGuard"
Cohesion: 0.12
Nodes (16): Async, Bear-trap checklist, Fail-open vs fail-closed, Financial actions and receipts, Guard modes, Integrating TrustLoopGuard, LLM/model route failures, MCP server (+8 more)

### Community 18 - "proxyRustJson"
Cohesion: 0.05
Nodes (40): POST(), DELETE(), PATCH(), GET(), POST(), POST(), RouteContext, proxyMock (+32 more)

### Community 19 - "Field-by-field"
Cohesion: 0.10
Nodes (21): 1. Putting banned vocabulary in `tone.forbidden`, 2. Listing categories instead of commitments in `authority.cannot_promise`, `agent_id`, Agent profile — field reference, `authority.can_promise`, `authority.cannot_promise`, `display_name`, `escalation_triggers` (+13 more)

### Community 22 - "plan.rs"
Cohesion: 0.15
Nodes (27): agent_disambiguator(), core_path(), core_vector(), delete_plan(), generate_static_policies(), id_slug(), list_plans(), plan_attack_vectors() (+19 more)

### Community 24 - "redteam.rs"
Cohesion: 0.06
Nodes (61): AttackVector, ComparedAttackStatus, CreateReportRequest, empty_json_object(), HardenCandidate, HardenCandidateOperation, HardenRejection, HardenRejectionReason (+53 more)

### Community 25 - "settings_update.rs"
Cohesion: 0.22
Nodes (23): app_with_owner(), environment_checker_modes_get_without_override_returns_all_inherit(), environment_checker_modes_round_trip(), get_request(), patch_settings_is_scoped_by_workspace_header(), patch_settings_rejects_invalid_mode_string(), patch_settings_rejects_non_numeric_retention_days(), patch_settings_rejects_unknown_default_action() (+15 more)

### Community 26 - "types.py"
Cohesion: 0.02
Nodes (217): BaseModel, Enum, AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone, AllowedSource (+209 more)

### Community 27 - "PostgresStore"
Cohesion: 0.18
Nodes (14): connect(), migrate(), PostgresStore, repair_known_schema_drift(), DbConnection, DbPool, Debug, Decision (+6 more)

### Community 28 - "Client"
Cohesion: 0.08
Nodes (11): browserRunContext(), Client, runContext(), RunContextStore, stringifyJson(), GuardEvent, GuardrailGenerateResponse, PolicyDocument (+3 more)

### Community 29 - "errors.ts"
Cohesion: 0.08
Nodes (25): CODE_TO_CLASS, codeFromHttpStatus(), Decode, DEFAULT_RETRIABLE, Forbidden, fromResponse(), Gone, Internal (+17 more)

### Community 36 - "TrustLoopGuard demos"
Cohesion: 0.25
Nodes (7): Agentic refund authorization, Bring your own agent, LiveKit, Money agent — guarded scenarios (flagship), NorthPay dispute, Stripe refund agent, TrustLoopGuard demos

### Community 37 - "Result"
Cohesion: 0.08
Nodes (58): action_from_record(), approval_from_record(), clean_operation(), clean_optional(), clean_required(), enum_from_text(), enum_text(), event_from_record() (+50 more)

### Community 38 - "AnalyticsChartGrid.tsx"
Cohesion: 0.04
Nodes (60): AnalyticsChartGrid(), AnalyticsChartGridProps, AnalyticsWidget(), applyGridOrder(), DEFAULT_LAYOUT, DEFAULT_VIEW, DIMENSION_LABELS, dimensionLabel() (+52 more)

### Community 39 - "param_auth.rs"
Cohesion: 0.10
Nodes (42): source(), allowed(), authority_param(), content_bearing_params_are_ignored(), content_param(), correct_source_yields_no_findings(), dangling_source_id_cannot_authorize(), empty_allowed_sources_rejects_all() (+34 more)

### Community 40 - "PostgresGatewayAdapter"
Cohesion: 0.13
Nodes (16): gateway_store_error(), PostgresGatewayAdapter, Arc, EnforcementProfile, EnforcementProfilePatch, GatewayProviderConnection, GatewayRoute, GatewayRoutePatch (+8 more)

### Community 42 - "llm_pricing.rs"
Cohesion: 0.07
Nodes (38): cost_minor(), default_table(), deployment_prefixes_suffix_match(), known_model_prices_exactly(), LlmPricingStoreError, LlmPricingTable, MemoryLlmPricingStore, BTreeMap (+30 more)

### Community 43 - "latest_review_outcomes"
Cohesion: 0.15
Nodes (20): latest_review_outcomes(), parse_review_outcome(), DateTime, DbConnection, DbPool, Debug, Formatter, HashMap (+12 more)

### Community 44 - "SDK-Driven Development at TrustLoopGuard"
Cohesion: 0.15
Nodes (13): Direct event submission, How features are built (the loop), MCP adapter, Out of scope, Publishing, Required CI gates, Reviewer checklist, Run grouping helper (+5 more)

### Community 47 - "tests.rs"
Cohesion: 0.05
Nodes (86): resolve_environment_id(), HeaderMap, Response, Result, String, workspace_id_from_headers(), harden_job(), HeaderMap (+78 more)

### Community 48 - "knowledge.rs"
Cohesion: 0.18
Nodes (15): knowledge_kind_text(), knowledge_row_to_document(), parse_knowledge_kind(), parse_knowledge_status(), PostgresKnowledgeAdapter, Arc, CreateKnowledgeSourceRequest, KnowledgeSourceDocument (+7 more)

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

### Community 56 - ".create_event"
Cohesion: 0.05
Nodes (53): CreateRunEventRequest, Result, RunEventSummary, Vec, RunRepo, CreateRunRequest, DbConnection, DbPool (+45 more)

### Community 57 - "agent.rs"
Cohesion: 0.13
Nodes (19): AgentAuthority, AgentScope, AgentTone, AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone (+11 more)

### Community 58 - "tests.rs"
Cohesion: 0.32
Nodes (11): allow_output(), default_runner_with_no_policies_yields_allow(), different_request_misses_cache(), empty_engine_allows(), req(), second_identical_request_hits_cache(), three_allow_tiers_yield_allow_with_three_results(), tier1_block_cancels_tiers_2_and_3() (+3 more)

### Community 60 - "scenarios.core.ts"
Cohesion: 0.14
Nodes (22): executePayment(), PaymentRequest, PaymentResult, simulatedLedger, StripePaymentIntent, assertEnforced(), main(), makeDecision() (+14 more)

### Community 61 - "report.rs"
Cohesion: 0.13
Nodes (33): ComparedAttackStatus, aggregate(), aggregates_exclude_clean_control_from_denominator(), blocked_and_clean_are_informational_with_no_evidence(), build_report(), categorize(), compared_attacks(), compared_status() (+25 more)

### Community 62 - "profile_record_to_wire"
Cohesion: 0.06
Nodes (42): GatewayRepo, EnforcementProfile, EnforcementProfilePatch, NewEnforcementProfile, Result, Vec, parse_fail_mode(), parse_input_action() (+34 more)

### Community 63 - "._run_with_retry"
Cohesion: 0.05
Nodes (31): RunListResponse, FinancialActionDecisionReceipt, FinancialActionListResponse, FinancialActionRecord, FinancialApprovalRequestListResponse, FinancialMandateListResponse, FinancialOutcomeListResponse, FinancialPolicyListResponse (+23 more)

### Community 64 - "Gateway Proxy Runtime Branch Guide"
Cohesion: 0.15
Nodes (12): Configuration Objects, Current Limits, Dashboard Proxy vs Runtime Proxy, Files to Read in Order, Gateway Proxy Runtime Branch Guide, How a Customer Routes Through the Proxy, One-Sentence Model, Provider Forwarding (+4 more)

### Community 65 - "EnvironmentStoreError"
Cohesion: 0.09
Nodes (29): EnvironmentStoreError, ensure_default(), MemoryEnvironmentStore, CreateWorkspaceEnvironmentRequest, HashMap, Result, RwLock, Self (+21 more)

### Community 66 - "properties"
Cohesion: 0.09
Nodes (22): properties, required, type, anyOf, Action, ToolMetadata, type, default (+14 more)

### Community 68 - "GuardEvent.ts"
Cohesion: 0.07
Nodes (30): GuardToolCallOptions, Action, AllowedSource, ApprovalRule, Confidentiality, EventKind, Integrity, LabelBasis (+22 more)

### Community 69 - "dashboard-data.ts"
Cohesion: 0.03
Nodes (152): ChangePasswordCard(), AccountPage(), AgentsPage(), AnalyticsPage(), AnalyticsSearchParams, ApiKeysPage(), escapeHeaderValue(), GET() (+144 more)

### Community 70 - "scripts"
Cohesion: 0.07
Nodes (29): scripts, build, codegen, codegen:check, coverage:backend, coverage:backend:lcov, coverage:frontend, dead-code:check (+21 more)

### Community 71 - "generate_guardrails"
Cohesion: 0.13
Nodes (26): draft_policy(), Bytes, Response, parse_policy_set(), policy_draft_item_schema(), policy_draft_json_schema(), policy_from_draft(), policy_set_draft_json_schema() (+18 more)

### Community 72 - "rustApiForAuthorizedWorkspace"
Cohesion: 0.09
Nodes (19): bodySchema, JsonValue, PATCH(), CreateApiKeyResponse, POST(), POST(), bodySchema, PATCH() (+11 more)

### Community 73 - "policies.ts"
Cohesion: 0.05
Nodes (34): aiEditPolicy(), aiEditResponseSchema, deletePolicy(), generatePolicyDraft(), generatePolicyDraftResponseSchema, getPolicyVersion(), listPolicyVersions(), ParsedPolicyDocument (+26 more)

### Community 75 - "PolicyStoreError"
Cohesion: 0.16
Nodes (16): PolicyStoreError, policy_action(), policy_summary_from_row(), PostgresPolicyAdapter, Action, Arc, EntityVersionDetail, EntityVersionListResponse (+8 more)

### Community 76 - "PolicyState"
Cohesion: 0.15
Nodes (28): batch_set_policy_enabled(), delete_policy(), get_policy(), list_policies(), parse_policy_family(), read_policy_family(), Bytes, HeaderMap (+20 more)

### Community 78 - "models.rs"
Cohesion: 0.07
Nodes (95): ApprovalRequestRecord, EnforcementProfileRecord, EntityVersionRecord, EscalationRecord, FinancialActionEventRecord, FinancialActionOutcomeRecord, FinancialActionRecord, FinancialLedgerEntryRecord (+87 more)

### Community 79 - "RunDetailLiveView.tsx"
Cohesion: 0.09
Nodes (39): buildGuardFlow(), buildRows(), DeliveryInterventionDetail(), DetailItem(), displayPolicy(), displayReason(), displayUserPrompt(), EventRow() (+31 more)

### Community 80 - "synthesis.rs"
Cohesion: 0.11
Nodes (42): action_candidate_backstop_matches_review_bypass_not_policy_questions(), Candidate, classifies_action_claim_from_reply_assertion(), classifies_configured_workflow_before_generic_action(), classifies_credential_from_reply_token(), classifies_pii_from_goal(), classifies_refund_workflow_before_generic_action(), classifies_system_prompt() (+34 more)

### Community 81 - "properties"
Cohesion: 0.10
Nodes (20): items, type, properties, required, type, items, type, ApprovalRule (+12 more)

### Community 82 - "_shared.ts"
Cohesion: 0.09
Nodes (28): GET(), RouteContext, POST(), RouteContext, GET(), GET(), GET(), DELETE() (+20 more)

### Community 83 - "index.ts"
Cohesion: 0.04
Nodes (48): DemoMetric, Metrics, percentile(), ClientOptions, Channel, CreateApiKeyRequest, CreateGatewayProviderConnectionRequest, CreateGatewayRouteRequest (+40 more)

### Community 84 - "AgentRepo"
Cohesion: 0.06
Nodes (46): AgentStoreError, MemoryAgentStore, AgentProfile, Arc, HashMap, Result, RwLock, Self (+38 more)

### Community 86 - "AuthConfig"
Cohesion: 0.09
Nodes (36): forwarded_user_id(), require_approved_user(), Option, Request, Response, Result, Uuid, AuthConfig (+28 more)

### Community 88 - "RunnerError"
Cohesion: 0.08
Nodes (28): RedteamPlanner, RedteamRunnerClient, Client, Error, Into, Option, Result, RunnerDispatch (+20 more)

### Community 89 - "Load test"
Cohesion: 0.29
Nodes (6): Load test, Prerequisites, Run, Scenarios, What's NOT here, What to look for

### Community 92 - "change_password"
Cohesion: 0.16
Nodes (21): AuthRequest, ChangePasswordRequest, change_password(), login(), Json, Response, signup(), change_password_same_as_current_is_400() (+13 more)

### Community 93 - "pipeline_e2e.rs"
Cohesion: 0.15
Nodes (38): approval_enforce_does_not_demote_an_engine_block(), approval_enforce_escalates_required_tool(), approval_enforce_ignores_tools_without_approval_rules(), approval_fixture(), approval_modes(), approval_off_records_nothing_and_decision_unchanged(), approval_shadow_records_hypothetical_escalate_without_changing_decision(), event_with_no_sources_and_no_provenance_yields_empty_evidence() (+30 more)

### Community 94 - "schema.rs"
Cohesion: 0.08
Nodes (28): ensure_oauth_user_exists(), ensure_user_exists(), generate_token(), invite_row_to_wire(), DbConnection, Result, String, Uuid (+20 more)

### Community 95 - "attacks-panel.tsx"
Cohesion: 0.02
Nodes (130): attackVectorSchema, dispatchBodySchema, documentTemplateSchema, isBase64(), validateDocumentTemplate(), workflowPathSchema, AttackButton(), AttackFlow() (+122 more)

### Community 96 - "RedteamState"
Cohesion: 0.10
Nodes (45): resolve_environment_id(), HeaderMap, Response, Result, String, cancel_job(), create_report(), dispatch_job() (+37 more)

### Community 97 - "gateway.rs"
Cohesion: 0.17
Nodes (18): build_app(), create_common_gateway_config(), create_workspace_key(), enable_streaming_mode(), gateway_owner_id(), json_request(), read_body(), read_text() (+10 more)

### Community 99 - "WorkspaceKeyContext"
Cohesion: 0.07
Nodes (84): ApiKeyBatchRevokeRequest, AnalyticsState, analytics_user_id(), AnalyticsUserId, authorize_analytics_workspace(), forwarded_user_id(), require_workspace_member(), Arc (+76 more)

### Community 100 - "run-detail-live.ts"
Cohesion: 0.11
Nodes (34): BASE_SNAPSHOT, defaultEventLabel(), eventSnapshot(), latestUserDisplayText(), objectSchema, parseRunDetailSnapshot(), readTracePolicy(), runDetailSnapshot (+26 more)

### Community 101 - "share.rs"
Cohesion: 0.11
Nodes (30): create_then_get_round_trips(), expired_share_reads_as_not_found(), generate_share_token(), is_expired(), MemoryRedteamReportShareStore, MemShare, new_share(), NewReportShare (+22 more)

### Community 103 - "checker_enforcement.rs"
Cohesion: 0.14
Nodes (45): all_none_override_inherits_workspace_modes(), app_with_approval_mode(), app_with_modes(), app_with_override(), approval_enforce_escalates_tool_requiring_approval(), approval_enforce_ignores_tools_without_approval_rules(), approval_escalation_enqueues_existing_worker_payload(), approval_shadow_keeps_decision_unchanged() (+37 more)

### Community 104 - "EventPipelineCtx"
Cohesion: 0.06
Nodes (42): MemoryChecker, Checker, CheckerFinding, composer_applies_worst_finding_and_copies_evidence_fields(), composer_ignores_signals_for_verdict(), composer_keeps_decision_when_no_finding_carries_a_verdict(), composer_never_downgrades_the_seeded_verdict(), composer_upgrades_rewrite_seed_and_preserves_it_against_weaker_findings() (+34 more)

### Community 105 - "FinancialAuthorizationService"
Cohesion: 0.12
Nodes (20): FinancialAuthorizationService, json_string_array_contains(), ledger_idempotency_key(), mandate_denial_reason(), mandate_scope_denial_reason(), CreateFinancialActionRequest, CreateFinancialMandateRequest, FinancialActionListResponse (+12 more)

### Community 106 - "path"
Cohesion: 0.10
Nodes (34): deadline_exceeded_yields_timeout(), malformed_inner_json_yields_parse_error(), non_2xx_yields_status_error(), ok_response(), openai_sends_bearer_auth_and_json_schema_body(), openrouter_adds_http_referer(), schema(), generate_404_maps_to_not_found() (+26 more)

### Community 107 - "Policy"
Cohesion: 0.11
Nodes (31): CheckRequest, CreateRunEventRequest, Default, RedactionInfo, absent_domain_defaults_to_customer_support(), agent_scope_matches(), channel_scope_matches(), domain_scope_matches() (+23 more)

### Community 108 - "normalization.rs"
Cohesion: 0.09
Nodes (38): seal_provider_key(), fail_mode_storage_text(), input_action_storage_text(), normalize_enforcement_profile(), normalize_enforcement_profile_patch(), normalize_gateway_route(), normalize_gateway_route_patch(), normalize_optional_text() (+30 more)

### Community 109 - "event_ingestion.rs"
Cohesion: 0.15
Nodes (38): app(), CannedLlmClient, CannedLlmResponse, direct_event_cannot_spoof_gateway_to_skip_run_stats(), direct_event_rejects_run_event_from_another_run(), direct_event_with_run_updates_run_stats(), full_evidence_flows_to_trace(), json_request() (+30 more)

### Community 110 - "SdkError"
Cohesion: 0.06
Nodes (56): Exception, CreateFinancialPolicyRequest, FinancialActionOutcome, FinancialPolicyRecord, Async variant of ``Client.create_financial_policy``., Create or update a financial spending control., code_from_http_status(), Decode (+48 more)

### Community 111 - "MemoryAnalyticsStore"
Cohesion: 0.16
Nodes (12): MemoryAnalyticsStore, AnalyticsDashboardView, AnalyticsFacetCatalogResponse, AnalyticsQueryRequest, AnalyticsQueryResponse, CreateAnalyticsDashboardViewRequest, HashMap, Result (+4 more)

### Community 112 - "redteam-report.ts"
Cohesion: 0.06
Nodes (36): COLORS, COMPARISON_STATUS, ComparisonSection(), Finding(), formatDate(), outcomeStyle(), pct(), ReportDocument() (+28 more)

### Community 113 - "tests.rs"
Cohesion: 0.20
Nodes (20): authority_violation_blocks(), CannedClient, ctx_with(), empty_router_yields_skipped(), FixedResolver, hallucination_violation_blocks(), no_profile_yields_skipped(), pre_cancelled_token_short_circuits() (+12 more)

### Community 116 - "page.tsx"
Cohesion: 0.07
Nodes (32): createKnowledgeSource(), readEnum(), readOptionalFile(), readOptionalString(), readRequiredString(), buildRetryUrl(), createWorkspace(), firstParam() (+24 more)

### Community 117 - "financial-actions.test.ts"
Cohesion: 0.08
Nodes (21): ACTION, DECISION_RECEIPT, FINANCIAL_POLICY, FINANCIAL_POLICY_REQUEST, MANDATE, MANDATE_REQUEST, OUTCOME, RECEIPT (+13 more)

### Community 118 - "AnalyticsDashboardWidget.ts"
Cohesion: 0.11
Nodes (18): AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView, AnalyticsDashboardViewConfig, AnalyticsDashboardViewListResponse, AnalyticsDashboardWidget, AnalyticsDimension (+10 more)

### Community 119 - "RedteamReportShareRepo"
Cohesion: 0.16
Nodes (16): NewShare, parse_uuid(), RedteamReportShareRepo, ReportShareRow, DateTime, DbConnection, DbPool, Debug (+8 more)

### Community 120 - "PostgresTraceAdapter"
Cohesion: 0.22
Nodes (10): PostgresTraceAdapter, Arc, Option, Result, Self, Sender, TraceSummary, Vec (+2 more)

### Community 121 - "Technical terms"
Cohesion: 0.06
Nodes (35): Attack plan, Attack runner, Attack vector, Cache key, Cold path, Decision log, Embedded mode, Escalation worker (+27 more)

### Community 122 - "tool-runner.ts"
Cohesion: 0.14
Nodes (25): testHeldActionDoesNotExecute(), testOfflineAgentApprovesAndExecutesProposedRefund(), testOrderSearch(), testOverRefundStillSubmitsFinancialAction(), testPrepareRefundBuildsTypedAction(), formatMoney(), prepareRefundTool(), searchOrderTool() (+17 more)

### Community 124 - "dashboard_admin_repo.rs"
Cohesion: 0.15
Nodes (27): DashboardAdminRepo, environment_checker_modes_from_record(), EnvironmentCheckerModesRecord, EnvironmentCheckerModesWriteRecord, mode_to_db(), optional_mode_to_db(), parse_data_handling_mode(), parse_enforcement_mode() (+19 more)

### Community 125 - "MemoryPolicyStore"
Cohesion: 0.13
Nodes (21): any_policy_document(), any_policy_summary(), normalize_policy_ids(), policy_action(), policy_document(), policy_summary(), Action, PolicyDocument (+13 more)

### Community 126 - "financial_authorization_service.rs"
Cohesion: 0.07
Nodes (62): executable_refund_request(), financial_policy(), mandate_request(), mandate_request_with_scope(), outcome(), payment_financial_policy(), payment_request(), refund_request() (+54 more)

### Community 127 - "family_parse.rs"
Cohesion: 0.10
Nodes (43): approval_requires_at_least_one_condition(), documented_family_examples_parse(), existing_content_examples_parse_via_load_any_str(), family(), family_id_uses_content_slug_rule(), family_less_yaml_parses_as_content_identical_to_load_str(), family_policies_round_trip_through_yaml_with_family_tag(), FamilyProbe (+35 more)

### Community 129 - "WorkspaceDashboard.tsx"
Cohesion: 0.06
Nodes (30): Badge(), badgeVariants, Verdict, VerdictLegend(), VerdictLegendProps, VERDICTS, BadgeVariant, PolicySeverityBadge() (+22 more)

### Community 130 - "Result"
Cohesion: 0.19
Nodes (11): insert_policy_version(), PolicyRepo, DbConnection, Option, PolicyFamily, PolicyRow, Result, String (+3 more)

### Community 131 - "client.ts"
Cohesion: 0.05
Nodes (37): ActiveRun, ActiveRunContext, buildFinancialOperationRequest(), cleanFinancialOperationField(), FinancialOperationRunOptions, FinancialOperationSpec, ListTracesOptions, WithRunOptions (+29 more)

### Community 133 - "HnswFuzzyChecker"
Cohesion: 0.12
Nodes (21): BuildError, dedup_when_both_tiers_match_same_policy(), empty_policies_yields_no_hits(), HnswFuzzyChecker, levenshtein_catches_typo_bypass(), levenshtein_misses_unrelated_text(), literal_policy(), Arc (+13 more)

### Community 134 - "policy_cli.rs"
Cohesion: 0.19
Nodes (21): Command, find_header_end(), policy_pull_writes_source_yaml_to_file(), policy_push_posts_yaml_to_server(), policy_push_rejects_family_yaml_with_clear_error(), policy_validate_reports_valid_family_yaml(), policy_validate_reports_valid_yaml(), read_http_request() (+13 more)

### Community 135 - "tl-client.ts"
Cohesion: 0.06
Nodes (45): GET(), MyWorkspace, MyWorkspacesResponse, POST(), userFromSession(), POST(), requestSchema, POST() (+37 more)

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
Cohesion: 0.15
Nodes (10): action(), apiAction(), MockFinancialRefundClient, timestamp(), MockRefundClient, timestamp(), PrepareRefundResult, CreateFinancialActionRequest (+2 more)

### Community 145 - "event_policy.rs"
Cohesion: 0.09
Nodes (55): all_literal_miss_does_not_call_semantic_judge(), any_literal_match_does_not_call_semantic_judge(), channel_name(), ClauseDecision, eval_ctx(), evaluate_event_policies(), evaluate_semantic_policy(), event_summary() (+47 more)

### Community 146 - "server.ts"
Cohesion: 0.04
Nodes (53): ClientEnv, createTrustLoopClient(), readClientOptions(), agentProfile(), createToolHandlers(), errorToolResult(), JsonObject, JsonPrimitive (+45 more)

### Community 147 - "tlClientForRequest"
Cohesion: 0.09
Nodes (25): POST(), RouteContext, POST(), RouteContext, POST(), requestSchema, DraftingClient, DraftResponse (+17 more)

### Community 149 - "api_error_response"
Cohesion: 0.14
Nodes (32): AgentState, delete_agent(), get_agent(), list_agents(), Arc, Bytes, HeaderMap, Option (+24 more)

### Community 151 - "package.json"
Cohesion: 0.22
Nodes (8): description, engines, node, license, name, packageManager, private, version

### Community 152 - "harden-job-card.tsx"
Cohesion: 0.11
Nodes (25): coverageLabel(), draftPolicyFromSessions(), HardenJobCard(), HardenJobCardProps, messageOf(), newPolicyHref(), operationLabel(), rejectionSummary() (+17 more)

### Community 153 - "RedteamJobStoreError"
Cohesion: 0.11
Nodes (28): event_text(), MemoryRedteamJobStore, HashMap, JobCounts, JobStatus, Option, RedteamAttackRecord, RedteamAttackRecordFilter (+20 more)

### Community 154 - "gateway_budget.rs"
Cohesion: 0.23
Nodes (40): actions_meter_policy_does_not_gate_llm_calls(), admin_request(), at_cap_denies_without_calling_upstream(), build_app(), chat_request(), create_common_gateway_config(), create_extra_runtime_key(), create_llm_budget() (+32 more)

### Community 155 - "guard.rs"
Cohesion: 0.12
Nodes (18): Channel, check_request_omits_absent_session_id_on_serialize(), Decision, RedactedEntity, RedactionInfo, RedactionMode, RedactionStatus, Into (+10 more)

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
Nodes (33): clean_optional(), clean_required(), key(), mandate_key(), MemoryFinancialStore, MemoryLedgerEntry, ApprovalRequirement, CreateFinancialActionRequest (+25 more)

### Community 161 - "properties"
Cohesion: 0.10
Nodes (20): type, $ref, type, properties, agent_id, authority, display_name, scope (+12 more)

### Community 162 - "type"
Cohesion: 0.15
Nodes (14): properties, default, items, type, default, items, type, default (+6 more)

### Community 163 - "labels.rs"
Cohesion: 0.14
Nodes (27): combine_all_trusted_is_trusted(), combine_any_untrusted_is_untrusted(), combine_confidentiality_takes_max_rank(), combine_integrity_takes_min_rank(), combine_labels(), combine_unknown_conf_outranks_public_only(), combine_unknown_without_untrusted_is_unknown(), confidentiality_rank() (+19 more)

### Community 164 - "Repository Agent Instructions"
Cohesion: 0.15
Nodes (12): Architecture: Rust Backend Is the Source of Truth, Coding Conventions, Docs Are the Single Source of Truth (`docs/concept`), General Coding Discipline, Goal-Driven Execution, Implementation Checklist, Page Integration Expectations, Repository Agent Instructions (+4 more)

### Community 165 - "latest_review_outcomes"
Cohesion: 0.09
Nodes (25): event_summary(), parse_reason_codes(), HumanReviewEvent, Result, String, Value, Vec, outcome_text() (+17 more)

### Community 166 - "ToolMetadataRepo"
Cohesion: 0.10
Nodes (33): cache_key(), deserialize_spec(), Arc, Cache, DbConnection, DbPool, Debug, Duration (+25 more)

### Community 167 - "TierResult"
Cohesion: 0.14
Nodes (14): TierResult, format, minimum, type, elapsed_ms, reasons, tier, default (+6 more)

### Community 168 - "tier_results"
Cohesion: 0.15
Nodes (13): items, type, $ref, entities, tier_results, triggered_policies, items, default (+5 more)

### Community 170 - "v0 Design Decisions"
Cohesion: 0.07
Nodes (30): 10. Crate alignment, 11. Build order (v0), 12. Open questions (need answers before phase 1), 13. Things deliberately not in v0, 14. Confirmation checklist, 15. Event-centered runtime (locked), 16. Enforcement is an opt-in rollout (locked), 17. Labeling strategy: structure-first, fail-closed for authority (locked) (+22 more)

### Community 171 - "Runtime Refactor Jobs"
Cohesion: 0.07
Nodes (28): Continuation Readability Pass, Current Status, Final Acceptance Gates, Phase 0: Baseline Evidence, Phase 1: Server Shell Cleanup, Phase 2: Guard Service Extraction, Phase 3: App State Decomposition, Phase 4: Gateway Decomposition (+20 more)

### Community 172 - "agents"
Cohesion: 0.18
Nodes (12): default, items, type, WhenClause, default, items, type, type (+4 more)

### Community 173 - "FinancialStoreError"
Cohesion: 0.08
Nodes (31): FinancialStoreError, String, financial_store_error(), PostgresFinancialAdapter, ApprovalRequirement, Arc, CreateFinancialActionRequest, CreateFinancialMandateRequest (+23 more)

### Community 174 - "in_scope"
Cohesion: 0.18
Nodes (11): properties, type, AgentScope, default, items, type, default, items (+3 more)

### Community 175 - "properties"
Cohesion: 0.08
Nodes (26): type, type, type, type, format, minimum, type, properties (+18 more)

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
Cohesion: 0.10
Nodes (50): ApprovalRequirement, CounterpartyRef, CreateFinancialActionRequest, CreateFinancialMandateRequest, CreateFinancialPolicyRequest, EvidenceRef, FinancialAction, FinancialActionDecision (+42 more)

### Community 191 - "JwtSigner"
Cohesion: 0.18
Nodes (20): access_token_carries_workspace_and_type(), Claims, JwtError, JwtSigner, rejects_garbage(), rejects_wrong_secret(), round_trip_mints_and_verifies(), Arc (+12 more)

### Community 194 - "pull_request_template.md"
Cohesion: 0.25
Nodes (7): 🔁 Cross-cutting concerns, 👀 Reviewer prompt, 🧩 SDK-parity checklist, 📝 Summary, ✅ Test plan, 🧭 Type of change, 🎨 UI Changes

### Community 195 - "core.ts"
Cohesion: 0.13
Nodes (17): buildRefundRequest(), createRefundMandate(), FinancialDemoClient, REFUND_SCENARIOS, RefundScenario, runRefundDemo(), runRefundScenario(), ScenarioKey (+9 more)

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
Cohesion: 0.10
Nodes (30): AnyPolicyRow, cache_key(), ensure_all_policies_exist(), ensure_policy_exists(), load_any_deployment_records(), PolicyRepo, Arc, DbConnection (+22 more)

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
Cohesion: 0.14
Nodes (28): ApprovalPolicy, ApprovalWhen, default_block_action(), default_escalate_action(), default_severity(), FamilyPolicy, FinancialPolicy, FinancialWhen (+20 more)

### Community 211 - "load_str"
Cohesion: 0.14
Nodes (27): matches_canonical_scope_fields(), skips_agent_scope_mismatch(), skips_domain_scope_mismatch(), accepts_canonical_scope_fields(), accepts_legacy_channel_scope_field(), content_family_tag_passes_load_str_directly(), documented_examples_parse(), format_issues() (+19 more)

### Community 213 - "core.ts"
Cohesion: 0.10
Nodes (15): buildRefundActionRequest(), ensureRefundMandate(), executeRefundTool(), messageForStatus(), normalizeReason(), normalizeRequestId(), providerReferenceFromReceipt(), REFUND_MANDATE_SCOPE (+7 more)

### Community 214 - "MemoryRunStore"
Cohesion: 0.13
Nodes (19): MemoryRunStore, p95_latency(), CreateRunEventRequest, CreateRunRequest, HashMap, Option, Result, RunEventSummary (+11 more)

### Community 216 - "RedteamJobStore"
Cohesion: 0.10
Nodes (37): RedteamJobStore, Send, Sync, DispatchConfig, DispatchJob, DispatchOutcome, drive(), is_cancelled() (+29 more)

### Community 220 - "Decision.ts"
Cohesion: 0.09
Nodes (19): CheckerFindingEvidence, CheckerRun, DataHandlingMode, EnforcementMode, EnvironmentCheckerModes, RFC-3339, RedactedEntity, RedactionInfo (+11 more)

### Community 222 - "validation.rs"
Cohesion: 0.14
Nodes (25): accepts_max_only_value_limit(), accepts_valid_value_limit(), metadata(), rejects_blank_allowed_source_id(), rejects_blank_approver_role(), rejects_blank_tool_name(), rejects_duplicate_param_paths(), rejects_empty_param_path() (+17 more)

### Community 223 - "dashboard.rs"
Cohesion: 0.18
Nodes (23): ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse, CreateApiKeyRequest, CreateApiKeyResponse, CreateWorkspaceEnvironmentRequest, DashboardApiKey, DataHandlingMode (+15 more)

### Community 224 - "policy.rs"
Cohesion: 0.08
Nodes (45): Args, main(), normalize_typescript(), patch_openapi_label_policy_upsert(), render_pydantic(), repo_root(), Option, Path (+37 more)

### Community 225 - "redteam_runner.rs"
Cohesion: 0.13
Nodes (35): empty_json_object(), RedteamRunnerContract, HashMap, Option, String, Value, Vec, runner_attack_surface_is_default() (+27 more)

### Community 228 - "AnyPolicy"
Cohesion: 0.22
Nodes (13): decode_policy_response(), load_policy_file(), pull_policy(), push_policy(), Option, PathBuf, PolicyDocument, Response (+5 more)

### Community 229 - "value_limit.rs"
Cohesion: 0.17
Nodes (23): absent_param_is_skipped(), allows_amount_at_max_boundary(), allows_amount_at_min_boundary(), allows_amount_under_max(), blocks_when_amount_below_min(), blocks_when_amount_exceeds_max(), bound_finding(), escalates_when_value_is_not_an_integer() (+15 more)

### Community 230 - "metrics.rs"
Cohesion: 0.07
Nodes (45): AnalyticsChartType, AnalyticsDimension, AnalyticsFilter, AnalyticsMetric, BTreeSet, AnalyticsRepo, AnalyticsFact, AnalyticsRepo (+37 more)

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
Cohesion: 0.16
Nodes (16): BudgetConfig, ConfigError, empty_budgets_section_uses_default(), ProviderConfig, ProviderTarget, round_trips_sample_config(), RouteConfig, RouterConfig (+8 more)

### Community 237 - "package.json"
Cohesion: 0.08
Nodes (24): bin, trustloopguard-mcp-server, dependencies, @modelcontextprotocol/sdk, @trustloopguard/sdk, zod, description, devDependencies (+16 more)

### Community 238 - "guarded_healthcare_agent.py"
Cohesion: 0.20
Nodes (11): Agent, blocked_reply(), entrypoint(), escalated_reply(), HealthcareAgent, log_guardrail(), Decision, JobContext (+3 more)

### Community 239 - "ProfileResolver"
Cohesion: 0.15
Nodes (16): FuzzyChecker, FuzzyHit, NoOpFuzzyChecker, NoOpProfileResolver, ProfileResolver, Action, AgentProfile, Arc (+8 more)

### Community 242 - "AgentProfile.ts"
Cohesion: 0.17
Nodes (10): UpsertAgentInput, AgentAuthority, AgentListResponse, AgentProfile, AgentScope, AgentTone, KnowledgeSource, KnowledgeSourceKind (+2 more)

### Community 244 - "README.md"
Cohesion: 0.10
Nodes (19): Backend tests, Built for, Choose your integration path, Contributing, Decision outcomes, Development setup, Documentation diagrams, Features (+11 more)

### Community 246 - "workflow_analyzer.rs"
Cohesion: 0.22
Nodes (17): adjacency(), analyze(), classify(), finds_source_to_sink_path_through_neutral_node(), lookalike_node_names_do_not_create_phantom_paths(), no_path_when_source_does_not_reach_sink(), node_types(), NodeRole (+9 more)

### Community 247 - "spawn_escalation_worker"
Cohesion: 0.14
Nodes (31): default_retry_policy_is_five_attempts(), deliver_one(), EscalationConfig, EscalationPayload, persist_pending(), RetryPolicy, Arc, Client (+23 more)

### Community 248 - "TeamStoreError"
Cohesion: 0.06
Nodes (42): generate_memory_token(), MemoryTeamState, MemoryTeamStore, AddMemberOutcome, MyWorkspace, Option, Result, RwLock (+34 more)

### Community 249 - "GatewayStoreError"
Cohesion: 0.19
Nodes (13): GatewayStoreError, MemoryGatewayStore, EnforcementProfile, EnforcementProfilePatch, GatewayProviderConnection, GatewayRoute, GatewayRoutePatch, NewEnforcementProfile (+5 more)

### Community 250 - "KnowledgeRepo"
Cohesion: 0.15
Nodes (18): KnowledgeFileRow, KnowledgeRepo, KnowledgeSourceRow, NewKnowledgeFile, NewKnowledgeSource, DateTime, DbConnection, DbPool (+10 more)

### Community 252 - "writer.rs"
Cohesion: 0.13
Nodes (25): build_trace_payload(), event(), flush(), DbPool, Decision, Default, Duration, GuardEvent (+17 more)

### Community 253 - "package.json"
Cohesion: 0.08
Nodes (23): dependencies, geist, next, react, react-dom, @t3-oss/env-nextjs, zod, devDependencies (+15 more)

### Community 254 - "LabelPolicyStoreError"
Cohesion: 0.23
Nodes (12): LabelPolicyStoreError, MemoryLabelPolicyStore, origin_key(), HashMap, Origin, Result, RwLock, Self (+4 more)

### Community 255 - "DashboardAdminStoreError"
Cohesion: 0.06
Nodes (45): WorkspaceApiKeyVerifyError, ApiKeyStore, DashboardAdminStoreError, memory_api_key_to_wire(), MemoryApiKeyRecord, MemoryApiKeyStore, MemorySettingsStore, normalize_ids() (+37 more)

### Community 256 - "create_knowledge_source"
Cohesion: 0.17
Nodes (17): create_knowledge_source(), get_knowledge_source_file(), list_knowledge_sources(), CreateKnowledgeSourceRequest, HeaderMap, Json, Path, Response (+9 more)

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

### Community 264 - "GatewayPageContent.tsx"
Cohesion: 0.05
Nodes (34): Tabs(), TabsContent(), TabsList(), tabsListVariants, TabsTrigger(), ActionBadge(), CredentialBadge(), EnumSelect() (+26 more)

### Community 265 - "event"
Cohesion: 0.07
Nodes (59): allows_trusted_public_flow_to_external_sink(), blocks_private_source_flowing_to_external_sink(), blocks_untrusted_controlled_high_impact_action(), emits_both_rules_when_both_violated(), escalates_dangling_provenance_source_ids(), escalates_missing_provenance_on_high_impact_action(), escalates_unattributed_provenance_paths(), escalates_unknown_trust_control_on_high_impact_action() (+51 more)

### Community 267 - "tool.rs"
Cohesion: 0.13
Nodes (23): AllowedSource, ApprovalRule, LimitAction, ParamLimit, ParamRole, ParamSpec, AllowedSource, ApprovalRule (+15 more)

### Community 268 - "output_safe_response"
Cohesion: 0.46
Nodes (12): finish_completed(), handle_output_enforcement(), handle_regeneration(), handle_rewrite_output(), output_safe_response(), OutputEnforcement, Decision, Option (+4 more)

### Community 269 - "label_policy.rs"
Cohesion: 0.24
Nodes (23): app(), delete_then_get_returns_not_found(), disabled_policy_listed_but_not_resolved(), disabled_policy_not_applied_at_runtime(), event_path_decision_unchanged_with_label_policies_configured(), event_request(), invalid_origin_path_rejected(), json_request() (+15 more)

### Community 271 - "hero.tsx"
Cohesion: 0.12
Nodes (8): HeroCard(), Hero(), LEDGER, LedgerRecord, VERDICT_COLOR, ITEMS, TrustBand(), TrustItem

### Community 273 - "properties"
Cohesion: 0.14
Nodes (14): type, RedactionInfo, type, $ref, context_redacted, input_redacted, mode, proposed_output_redacted (+6 more)

### Community 276 - "harden.rs"
Cohesion: 0.13
Nodes (26): Send, Sync, SemanticPolicyJudge, candidate_source(), ClassGroup, is_control(), load_workflow_requirements(), match_has_semantic() (+18 more)

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
Nodes (31): ChannelTraceStore, list_traces(), MemoryTraceStore, read_query_param(), Arc, DateTime, Decision, GuardEvent (+23 more)

### Community 281 - "EnvironmentRepo"
Cohesion: 0.18
Nodes (14): clear_default(), environment_to_wire(), EnvironmentRepo, CreateWorkspaceEnvironmentRequest, DbConnection, DbPool, Debug, Formatter (+6 more)

### Community 282 - "Result"
Cohesion: 0.24
Nodes (13): any_policy_row_from_record(), policy_family_from_storage(), policy_from_json(), policy_from_storage(), policy_row_from_record(), PolicyRepo, Arc, Option (+5 more)

### Community 283 - "Contributing to TrustLoopGuard"
Cohesion: 0.25
Nodes (8): Commit style, Contributing to TrustLoopGuard, Development setup, License, Proposing changes, Pull request checklist, Reporting bugs, The three SDK-driven rules

### Community 284 - "RedteamPlanRepo"
Cohesion: 0.17
Nodes (16): parse_uuid(), plan_response(), PlanBody, RedteamPlanRepo, AttackVector, DbConnection, DbPool, Debug (+8 more)

### Community 285 - "Engine"
Cohesion: 0.13
Nodes (17): Engine, Arc, Self, Vec, OrchestrateConfig, Default, Duration, Self (+9 more)

### Community 287 - "definitions"
Cohesion: 0.11
Nodes (18): definitions, RunnerAttackSurface, RunnerRunMode, RunnerSessionEvent, RunnerStatus, description, enum, type (+10 more)

### Community 288 - "redteam-core.ts"
Cohesion: 0.13
Nodes (17): ALLOWED_AGENT_HOSTS, REDTEAM_PROFILES, RedteamCase, redteamCaseSchema, redteamLlmSchema, redteamOutcomeSchema, redteamProfileSchema, RedteamReport (+9 more)

### Community 291 - "ReviewQueueContent.tsx"
Cohesion: 0.11
Nodes (23): CardAction(), ReviewActionDialog(), ReviewActionDialogProps, Verdict, Filter, FILTERS, isActionableVerdict(), OUTCOME_LABEL (+15 more)

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

### Community 298 - "human_review.rs"
Cohesion: 0.18
Nodes (12): PostgresHumanReviewAdapter, Arc, CreateHumanReviewEventRequest, HumanReviewAnalyticsFilter, HumanReviewAnalyticsResponse, HumanReviewEvent, Option, Result (+4 more)

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
Nodes (21): duplicate_request_id_is_a_noop(), event(), event_matches(), grouped_usage_by_day_uses_utc_date_key(), grouped_usage_folds_by_principal_and_model(), MemoryLlmUsageStore, DateTime, LlmUsageBucketsResponse (+13 more)

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
Cohesion: 0.21
Nodes (11): GatewayPageContentData, shell, CreateEnforcementProfileRequest, EnforcementProfile, EnforcementProfileListResponse, FailMode, GatewayInputAction, GatewayOutputAction (+3 more)

### Community 314 - "tests.rs"
Cohesion: 0.23
Nodes (13): create_path_accepts_family_policies(), family_policy_json_validates_through_endpoint_path(), family_policy_yaml_validates_through_endpoint_path(), invalid_family_policy_returns_structured_issues_and_id(), load_str_and_validate_endpoint_agree_on_valid_yaml(), malformed_yaml_returns_validation_issue(), HeaderMap, unknown_family_is_invalid_with_truncated_echo() (+5 more)

### Community 315 - "gateway_repo.rs"
Cohesion: 0.16
Nodes (14): EnforcementProfilePatch, GatewayProviderConnectionSecret, GatewayRepo, GatewayRoutePatch, ResolvedGatewayRoute, DbConnection, DbPool, EnforcementProfile (+6 more)

### Community 316 - "knip.json"
Cohesion: 0.18
Nodes (10): ignore, ignoreBinaries, ignoreDependencies, ignoreFiles, ignoreIssues, apps/docs/source.config.ts, apps/web/components/ui/**, sdks/typescript/src/generated/** (+2 more)

### Community 317 - "Code of Conduct"
Cohesion: 0.33
Nodes (5): Attribution, Code of Conduct, Enforcement, Our Pledge, Our Standards

### Community 318 - "PostgresLabelPolicyAdapter"
Cohesion: 0.19
Nodes (16): label_policy_store_error(), origin_key(), PostgresLabelPolicyAdapter, Arc, Option, Origin, PolicyRepo, Result (+8 more)

### Community 319 - "RetryConfig"
Cohesion: 0.05
Nodes (69): GuardModeInput, OnAllowAsync, OnBlockAsync, OnErrorAsync, OnEscalateAsync, OnReviseAsync, RateLimited, Channel (+61 more)

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

### Community 325 - "LabelPolicyProvider"
Cohesion: 0.12
Nodes (18): LabelPolicyProvider, LabelPolicyUnavailable, NoOpLabelPolicyProvider, PolicyLabelResolver, ProvenancePropagator, Arc, GuardEvent, Result (+10 more)

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
Cohesion: 0.19
Nodes (14): RunStoreError, PostgresRunAdapter, Arc, CreateRunEventRequest, CreateRunRequest, Result, RunEventSummary, RunSummary (+6 more)

### Community 331 - "evaluate_financial_policies"
Cohesion: 0.15
Nodes (29): action_verdict(), compose(), evaluate_financial_policies(), financial_windowed_verdict(), per_action_verdicts(), Action, FinancialAction, I (+21 more)

### Community 332 - "StorageError"
Cohesion: 0.09
Nodes (40): AnalyticsRepo, clear_default(), ensure_view_exists(), AnalyticsDashboardView, CreateAnalyticsDashboardViewRequest, DbConnection, Result, UpdateAnalyticsDashboardViewRequest (+32 more)

### Community 333 - "tests.rs"
Cohesion: 0.27
Nodes (9): memory_store_delete_then_get_not_found(), memory_store_list_sorted(), memory_store_round_trip(), profile(), AgentProfile, validate_accepts_small_workflow_definition(), validate_rejects_empty_agent_id(), validate_rejects_empty_in_scope() (+1 more)

### Community 334 - "GuardEvent Redaction Spec"
Cohesion: 0.10
Nodes (19): 1. SDK-local redaction, 2. Customer-environment redaction service, 3. Server-side redaction, Acceptance Criteria, Deployment Modes, Goals, GuardEvent Redaction Spec, Hosted Cloud Behavior (+11 more)

### Community 335 - "embedder.rs"
Cohesion: 0.16
Nodes (16): cosine(), EmbedError, FastEmbedder, fnv1a(), mock_embedder_is_deterministic(), mock_embedder_normalises_to_unit(), MockEmbedder, Default (+8 more)

### Community 337 - "route.ts"
Cohesion: 0.15
Nodes (11): GET(), PUT(), RouteContext, stringListSchema, AGENT, MockRustApiError, MockWorkspaceAccessError, rustMock (+3 more)

### Community 339 - "financial_actions.rs"
Cohesion: 0.22
Nodes (28): app(), app_for(), create_payment_connection(), financial_action_decision_receipt_explains_held_refund(), financial_action_decision_receipt_missing_action_returns_404(), financial_action_outcomes_record_and_list(), financial_actions_create_get_and_transition(), financial_actions_list_workspace_actions() (+20 more)

### Community 340 - "create_review_event"
Cohesion: 0.27
Nodes (12): create_review_event(), human_review_analytics(), list_review_events(), CreateHumanReviewEventRequest, HeaderMap, Json, Path, Response (+4 more)

### Community 341 - "AppState"
Cohesion: 0.05
Nodes (81): AgentStore, Send, Sync, AnalyticsStore, Send, Sync, agent_routes(), analytics_routes() (+73 more)

### Community 342 - "financial_actions_integration.rs"
Cohesion: 0.15
Nodes (26): action_body(), decision_receipt_body(), financial_action_helpers_encode_ids_and_parse_statuses(), financial_mandate_helpers_create_list_and_revoke(), financial_outcome_helpers_record_and_list(), financial_policy_body(), financial_policy_helpers_create_and_list_controls(), financial_policy_request() (+18 more)

### Community 343 - "team.rs"
Cohesion: 0.20
Nodes (15): CreateInviteRequest, CreateInviteResponse, CreateWorkspaceRequest, InviteListResponse, InviteStatus, MemberListResponse, MyWorkspace, MyWorkspacesResponse (+7 more)

### Community 344 - "executor.rs"
Cohesion: 0.18
Nodes (20): FinancialExecutionError, FinancialExecutionResult, FinancialExecutor, PaymentHttpFinancialExecutor, provider_body(), recovery_status(), reversal_capability(), Arc (+12 more)

### Community 345 - "lib.rs"
Cohesion: 0.18
Nodes (19): create_environment(), delete_environment(), environment_id_from_headers(), EnvironmentState, list_environments(), resolve_environment_id(), environment_error_response(), Response (+11 more)

### Community 347 - "analytics.rs"
Cohesion: 0.24
Nodes (23): AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView, AnalyticsDashboardViewConfig, AnalyticsDashboardViewListResponse, AnalyticsDashboardWidget, AnalyticsDimension (+15 more)

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

### Community 353 - "patch_enforcement_profile"
Cohesion: 0.11
Nodes (34): create_enforcement_profile(), list_enforcement_profiles(), patch_enforcement_profile(), CreateEnforcementProfileRequest, Extension, HeaderMap, Json, Option (+26 more)

### Community 354 - "components.json"
Cohesion: 0.11
Nodes (17): aliases, components, hooks, lib, ui, utils, iconLibrary, rsc (+9 more)

### Community 355 - "policy_repo.rs"
Cohesion: 0.29
Nodes (15): batch_set_enabled_is_atomic_for_missing_policy(), batch_set_enabled_updates_all_selected_policies(), fresh_repo(), list_enabled_filters_disabled_and_deleted(), missing_policy_returns_not_found(), ContainerAsync, PolicyRepo, PostgresImage (+7 more)

### Community 356 - "RunSummary"
Cohesion: 0.15
Nodes (12): CreateRunRequest, RunStatus, RunSummary, UpdateRunRequest, Async variant of ``Client.start_run``., Async variant of ``Client.update_run``., Async variant of ``Client.finish_run``., Create a run grouping for subsequent ``check`` calls. (+4 more)

### Community 357 - "effective_checker_modes"
Cohesion: 0.19
Nodes (18): checker_run_evidence(), CheckerModes, CheckerRun, EnforcementMode, all_none_override_inherits_workspace_modes(), checker_modes(), effective_checker_modes(), no_override_inherits_workspace_modes() (+10 more)

### Community 358 - "severity"
Cohesion: 0.67
Nodes (3): severity, allOf, default

### Community 359 - "MemoryKnowledgeStore"
Cohesion: 0.22
Nodes (9): MemoryKnowledgeStore, HashMap, KnowledgeSourceDocument, KnowledgeSourceFileResponse, Result, RwLock, Self, String (+1 more)

### Community 360 - "LlmClient"
Cohesion: 0.29
Nodes (16): LlmClient, Send, Sync, build_budget(), build_provider(), build_providers(), build_routes(), ensure_provider_exists() (+8 more)

### Community 361 - "tests.rs"
Cohesion: 0.26
Nodes (13): missing_route_yields_http_error(), MockClient, no_fallback_propagates_primary_error(), over_budget_blocks_request_before_calling_provider(), primary_failure_falls_back_to_secondary(), primary_success_records_budget_and_skips_fallback(), Arc, AtomicUsize (+5 more)

### Community 362 - "setup.ts"
Cohesion: 0.14
Nodes (20): guardedPayout(), headers(), main(), registerTool(), AGENT_DEMO_WORLD_PORT, createClient(), demoRoot(), fetchWithWorkspace() (+12 more)

### Community 363 - "test_financial_actions.py"
Cohesion: 0.14
Nodes (25): action_body(), approval_body(), decision_receipt_body(), financial_policy_body(), financial_policy_request(), mandate_body(), mandate_request(), outcome() (+17 more)

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
Cohesion: 0.33
Nodes (7): OpenRouterClient, Client, Duration, Into, Result, Self, String

### Community 372 - "resolved_event"
Cohesion: 0.19
Nodes (16): ApprovalChecker, empty_roles_fall_back_to_generic_remediation(), escalates_when_tool_requires_approval(), metadata(), no_approval_rule_emits_nothing(), not_required_emits_nothing(), registry_reason_wins_over_generated_remediation(), remediation() (+8 more)

### Community 374 - "TokenBudget"
Cohesion: 0.17
Nodes (14): BudgetExceeded, BudgetState, exceeding_default_limit_errors(), HashMap, Into, Mutex, Result, Self (+6 more)

### Community 375 - "load_agent_str"
Cohesion: 0.21
Nodes (15): load_agent_str(), AgentProfile, Result, loads_committed_fixture_acme_support_v3(), parses_full_featured_profile(), parses_minimal_profile(), parses_web_knowledge_source_metadata(), rejects_duplicate_knowledge_source_ids() (+7 more)

### Community 376 - "monitoring_integration.rs"
Cohesion: 0.25
Nodes (16): allow_decision(), caller_explicit_session_is_never_overwritten(), client_without_monitoring_sends_no_session_id(), event(), mock_post(), monitoring_client_tags_submitted_events_with_session(), one_shot_retry(), record_event_delivers_without_blocking() (+8 more)

### Community 377 - "validation.rs"
Cohesion: 0.33
Nodes (9): CreateRunEventRequest, CreateRunRequest, Result, UpdateRunRequest, Value, validate_create_run(), validate_create_run_event(), validate_metadata() (+1 more)

### Community 378 - "ReportRateLimiter"
Cohesion: 0.16
Nodes (13): allows_up_to_max_then_blocks(), keys_are_independent(), ReportRateLimiter, resets_after_window(), Debug, Duration, Formatter, HashMap (+5 more)

### Community 379 - "guardrails.rs"
Cohesion: 0.24
Nodes (15): build_app(), delete_agent_cascades_to_owned_policies(), generate_for_missing_agent_is_404(), generate_persists_each_draft_disabled_and_returns_them(), generate_without_system_prompt_is_422(), list_for_unknown_agent_returns_empty(), list_returns_policies_scoped_to_agent(), read_body() (+7 more)

### Community 380 - "financial_repo.rs"
Cohesion: 0.18
Nodes (21): approval_requests_are_tenant_scoped_and_newest_first(), create_action_is_idempotent_and_tenant_scoped(), fresh_pool(), list_actions_is_tenant_scoped_and_newest_first(), mandate_request(), mandates_create_list_and_revoke_are_tenant_scoped(), outcome(), outcomes_append_and_list_by_action_without_affecting_spend() (+13 more)

### Community 381 - "module_exports.rs"
Cohesion: 0.11
Nodes (13): CreateHumanReviewEventRequest, RFC-3339, HumanReviewEvent, RFC-3339, HumanReviewEventListResponse, HumanReviewOutcome, RunDetail, RunEventKind (+5 more)

### Community 382 - "Crates"
Cohesion: 0.12
Nodes (17): Adding a new crate, Crates, Current Boundary Decisions, Dependency graph, `tl-cache` — decision cache, `tl-cli` — operator command line, `tl-codegen` — derived-artifact generator, `tl-core` — the type backbone (+9 more)

### Community 383 - "redteam-runner.schema.json"
Cohesion: 0.12
Nodes (16): description, $ref, $ref, $ref, $ref, properties, dispatch, handle (+8 more)

### Community 384 - "test_events.py"
Cohesion: 0.24
Nodes (14): TrustLoopGuard Python SDK.  Public surface:     Client          — HTTP client fo, Retry policy for the TrustLoopGuard Python SDK.  Mirrors `tl-sdk-rust`'s `RetryC, default_allow_decision(), GuardEvent, submit_event tests: typed round trip + error mapping, sync and async., run_event_summary(), run_summary(), send_email_event() (+6 more)

### Community 385 - "PostgresAnalyticsAdapter"
Cohesion: 0.15
Nodes (13): AnalyticsRepo, analytics_store_error(), PostgresAnalyticsAdapter, AnalyticsDashboardView, AnalyticsFacetCatalogResponse, AnalyticsQueryRequest, AnalyticsQueryResponse, Arc (+5 more)

### Community 387 - "docs-auth.ts"
Cohesion: 0.22
Nodes (11): POST(), redirectTo(), POST(), redirectTo(), UnlockPage(), UnlockPageProps, createDocsAuthToken(), safeDocsRedirectPath() (+3 more)

### Community 388 - "scripts"
Cohesion: 0.10
Nodes (21): scripts, arena:check, dev, dispute, dispute:byo, dispute:check, dispute:scenarios, dispute:scenarios:check (+13 more)

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
Cohesion: 0.33
Nodes (10): AgentsCmd, Cli, Cmd, GuardrailsCmd, main(), PolicyCmd, Option, PathBuf (+2 more)

### Community 395 - "env.ts"
Cohesion: 0.09
Nodes (28): AuthScreen(), FormErrorProps, OrDivider(), Spinner(), SpinnerProps, CredentialsForm(), OAuthButtons(), OAuthButtonsProps (+20 more)

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
Cohesion: 0.16
Nodes (19): build_policy_draft_llm(), router(), Arc, Option, memory_app_state(), analytics_catalog_query_and_saved_views_round_trip(), analytics_endpoints_are_protected_by_bearer_auth(), internal_bearer_analytics_requires_forwarded_workspace_member() (+11 more)

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

### Community 415 - "PolicyError"
Cohesion: 0.30
Nodes (14): is_private_host(), is_private_ip(), public_url_error(), AgentProfile, Result, String, validate(), validate_knowledge_sources() (+6 more)

### Community 416 - "compilerOptions"
Cohesion: 0.14
Nodes (13): compilerOptions, allowJs, exactOptionalPropertyTypes, incremental, jsx, lib, noEmit, paths (+5 more)

### Community 417 - "verify_candidate"
Cohesion: 0.19
Nodes (18): candidate_that_false_blocks_a_control_does_not_pass(), candidate_that_misses_a_variant_does_not_pass(), fires(), KeywordJudge, output_event(), policy(), regex_candidate_verifies_without_a_judge(), GuardEvent (+10 more)

### Community 418 - "PostgresUserAdapter"
Cohesion: 0.22
Nodes (10): PostgresUserAdapter, Arc, Result, Self, UserRecord, Uuid, user_record_from_row(), user_store_create_error() (+2 more)

### Community 419 - "types.ts"
Cohesion: 0.19
Nodes (21): customerBackendState(), ensureOrderDatabase(), findOrder(), listOrders(), listRefunds(), nullableTextValue(), numberValue(), openDatabase() (+13 more)

### Community 421 - "OpenAiClient"
Cohesion: 0.33
Nodes (7): OpenAiClient, Client, Duration, Into, Result, Self, String

### Community 422 - "fresh_store"
Cohesion: 0.15
Nodes (20): new_trace_id(), HumanReviewOutcome, Option, String, Value, Vec, TraceListResponse, TraceSummary (+12 more)

### Community 423 - "seal_key_material"
Cohesion: 0.21
Nodes (12): build_seal_key(), Option, Result, String, seal_key_config_requires_secret_without_explicit_dev_override(), seal_key_material(), unseal_provider_key(), env_filter() (+4 more)

### Community 424 - "properties"
Cohesion: 0.10
Nodes (20): type, Principal, type, properties, required, type, agent_id, environment_id (+12 more)

### Community 425 - "openai-agent.ts"
Cohesion: 0.16
Nodes (13): runRefundAgent(), shouldUseOpenAI(), main(), promptFromArgsOrStdin(), AgentState, initialMessages(), nextAssistantMessage(), runOpenAiRefundAgent() (+5 more)

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
Cohesion: 0.05
Nodes (42): enum, type, definitions, Confidentiality, EnforcementMode, EventKind, Integrity, LabelBasis (+34 more)

### Community 432 - "MemoryToolMetadataStore"
Cohesion: 0.09
Nodes (33): PostgresToolMetadataAdapter, Arc, Option, Result, Self, ToolMetadata, ToolMetadataEntry, Vec (+25 more)

### Community 433 - "Human Review Analytics Spec"
Cohesion: 0.14
Nodes (13): Acceptance Criteria, API Contract, Dashboard UX, Data Model, Definitions, Goals, Human Review Analytics Spec, Implementation Scope (+5 more)

### Community 434 - "WorkflowRequirement"
Cohesion: 0.14
Nodes (14): WorkflowRequirement, type, name, required_before, sensitive_steps, default, items, type (+6 more)

### Community 435 - "HumanReviewAnalyticsResponse.ts"
Cohesion: 0.21
Nodes (7): HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary, HumanReviewGroupRow, HumanReviewOutcomeCounts, HumanReviewPolicyRow, HumanReviewReasonRow, HumanReviewWorkflowStepRow

### Community 436 - ".submit_event"
Cohesion: 0.33
Nodes (6): _merge_context(), Decision, GuardEvent, SideEffectClass, Submit a full ``GuardEvent`` (sources + provenance) for a runtime decision., Submit a full ``GuardEvent`` (sources + provenance) for a runtime decision.

### Community 437 - "compilerOptions"
Cohesion: 0.15
Nodes (12): compilerOptions, allowJs, incremental, jsx, lib, noEmit, paths, plugins (+4 more)

### Community 438 - "devDependencies"
Cohesion: 0.15
Nodes (13): devDependencies, jsdom, tailwindcss, @tailwindcss/postcss, @testing-library/jest-dom, @testing-library/react, @testing-library/user-event, @types/node (+5 more)

### Community 439 - "LlmRouter"
Cohesion: 0.20
Nodes (11): JudgeKind, LlmRouter, ResolvedRoute, Arc, Debug, Formatter, HashMap, Option (+3 more)

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

### Community 450 - "mod.rs"
Cohesion: 0.15
Nodes (14): password_auth_enabled_from_env(), password_auth_enabled_from_values(), Option, build_app_state(), build_escalation_worker(), build_llm_router(), load_policies(), Arc (+6 more)

### Community 451 - "request"
Cohesion: 0.13
Nodes (19): build_app(), create_json_policy_canonicalizes_source_yaml(), create_then_get_policy_round_trips_source_yaml(), batch_disable_missing_policy_does_not_partially_update(), batch_disable_updates_multiple_policies(), delete_policy_makes_get_return_404(), disable_policy_updates_document_but_get_still_works(), disable_policy_with_malformed_json_returns_api_error() (+11 more)

### Community 453 - "lib.rs"
Cohesion: 0.09
Nodes (24): DecisionStore, Send, Sync, MemoryStore, Arc, Decision, HashMap, Result (+16 more)

### Community 454 - "RunnerPlanRequest"
Cohesion: 0.15
Nodes (13): type, RunnerPlanRequest, agentDisplayName, systemPrompt, workflowPresent, additionalProperties, description, properties (+5 more)

### Community 455 - "$ref"
Cohesion: 0.15
Nodes (13): description, items, type, default, items, type, $ref, default (+5 more)

### Community 457 - "SourceLabelPolicy"
Cohesion: 0.21
Nodes (10): Confidentiality, Integrity, Option, Origin, Trust, Vec, SourceLabelPolicy, SourceLabelPolicyEntry (+2 more)

### Community 458 - "AnalyticsStoreError"
Cohesion: 0.42
Nodes (9): AnalyticsStoreError, UpdateAnalyticsDashboardViewRequest, AnalyticsDashboardViewConfig, AnalyticsWidgetLayout, Result, validate_config(), validate_layout(), validate_name() (+1 more)

### Community 459 - "retry_integration.rs"
Cohesion: 0.36
Nodes (11): does_not_retry_401(), event(), fast_retry(), gives_up_after_max_attempts(), honors_retry_after_header(), ok_decision_body(), retries_503_until_success(), GuardEvent (+3 more)

### Community 460 - "FinancialPolicyRecord.ts"
Cohesion: 0.08
Nodes (23): TriggeredPolicy, Vec, Tier, TierResult, TierStatus, CreateFinancialPolicyRequest, FinancialActionPrecondition, FinancialDecisionRisk (+15 more)

### Community 461 - "llm_usage.rs"
Cohesion: 0.21
Nodes (17): list_llm_usage(), llm_usage_error_response(), LlmUsageFilter, LlmUsageGroupBy, LlmUsageState, parse_rfc3339(), read_query(), Arc (+9 more)

### Community 462 - "page.tsx"
Cohesion: 0.19
Nodes (15): createPolicy(), createPolicyErrorMessage(), initialPolicyValues(), NewPolicyPage(), PolicyValidationResult, readEnumOrNull(), readOptionalString(), readSearchString() (+7 more)

### Community 463 - "check_and_maybe_regenerate"
Cohesion: 0.13
Nodes (20): check_gateway_content(), GatewayContentCheck, GatewayDecisionLog, log_gateway_decision(), Decision, Option, ResolvedGatewayRoute, Response (+12 more)

### Community 464 - "MemoryHumanReviewStore"
Cohesion: 0.18
Nodes (14): empty_analytics(), key(), MemoryHumanReviewStore, CreateHumanReviewEventRequest, HashMap, HumanReviewAnalyticsFilter, HumanReviewAnalyticsResponse, HumanReviewEvent (+6 more)

### Community 465 - "route.test.ts"
Cohesion: 0.32
Nodes (5): GET(), POST(), RouteContext, proxyMock, RouteContext

### Community 466 - "analytics.rs"
Cohesion: 0.12
Nodes (27): count_outcome(), group_row(), GroupAccumulator, is_human_intervention(), payload_string(), percentage(), policy_ids(), PolicyAccumulator (+19 more)

### Community 468 - "budget.rs"
Cohesion: 0.23
Nodes (16): admit_llm_budget(), budget_exceeded_response(), llm_budget_policy_matches(), meter_llm_usage(), monday_is_its_own_week_start(), month_rollover_resets_day_and_month_but_not_week(), principal_for(), DateTime (+8 more)

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
Cohesion: 0.60
Nodes (4): forwardToWebhook(), hits, isRateLimited(), POST()

### Community 473 - "redaction"
Cohesion: 0.67
Nodes (3): redaction, anyOf, default

### Community 475 - "ConnectAgentStep.tsx"
Cohesion: 0.08
Nodes (32): ConnectAgentStep(), FirstEventStatus(), FLOW_BEATS, NEXT_STEPS, onboardingContextQuery(), CREATED, CopyBlock(), useFirstTrace() (+24 more)

### Community 476 - "context.rs"
Cohesion: 0.29
Nodes (9): generate_guardrails(), list_guardrails(), GuardrailGenerateResponse, GuardrailListResponse, Option, Result, String, run_agents() (+1 more)

### Community 477 - "Embedder"
Cohesion: 0.20
Nodes (8): Run The Chat Demo, Start The Server, Try Gateway Mode, Try The Demo Surfaces, Write Your First Policy, Embedder, Send, Sync

### Community 478 - "tests.rs"
Cohesion: 0.27
Nodes (16): assert_human_review_schema_exists(), assert_human_review_schema_missing(), assert_legacy_orphan_trace_preserved(), assert_migration_was_recorded(), assert_relation_state(), drop_human_review_schema(), establish(), fresh_database_url() (+8 more)

### Community 479 - "route.test.ts"
Cohesion: 0.33
Nodes (4): GET(), RouteContext, proxyMock, RouteContext

### Community 480 - "Architecture"
Cohesion: 0.18
Nodes (11): Architecture, Customer integration paths, Dashboard-owned surfaces, End-state to keep in mind, Event-centered check model, Latency budget (committed), Request lifecycle (HTTP path), Runtime data flow (+3 more)

### Community 481 - "Team & invites"
Cohesion: 0.18
Nodes (11): Acceptance flow, Authorization model, Endpoints, Enforcement, Invite lifecycle, Memory mode, Ownership, Roles (+3 more)

### Community 484 - "ui.ts"
Cohesion: 0.23
Nodes (14): AgentRunLogEntry, AgentRunResult, CustomerBackendState, ChatRequest, ChatResponse, escapeHtml(), handleChat(), handleRequest() (+6 more)

### Community 485 - "compilerOptions"
Cohesion: 0.20
Nodes (9): compilerOptions, declaration, lib, outDir, rootDir, types, exclude, extends (+1 more)

### Community 486 - "parse_body"
Cohesion: 0.17
Nodes (13): api_error_response(), ApiErrorCode, Response, StatusCode, String, is_yaml_content_type(), parse_body(), AgentProfile (+5 more)

### Community 487 - "events_integration.rs"
Cohesion: 0.38
Nodes (9): observe_only_decision(), one_shot_retry(), GuardEvent, RetryConfig, Value, run_scoped_client_attaches_run_and_event_ids(), send_email_event(), submit_event_maps_server_error() (+1 more)

### Community 489 - "package.json"
Cohesion: 0.12
Nodes (15): dependencies, openai, pdfjs-dist, @trustloopguard/sdk, yaml, description, devDependencies, tsx (+7 more)

### Community 491 - "fresh_repo"
Cohesion: 0.27
Nodes (10): api_key_principal_round_trips_create_list_verify(), batch_revoke_api_keys_is_workspace_scoped(), batch_revoke_api_keys_updates_status_and_auth_lookup(), checker_mode_check_constraint_rejects_invalid_values(), fresh_repo(), get_settings_round_trips_checker_enforcement_modes(), ContainerAsync, DashboardAdminRepo (+2 more)

### Community 492 - "gateway.mdx"
Cohesion: 0.20
Nodes (9): Anthropic clients, Configuration model, Current limits, Enforcement signals, OpenAI-compatible clients, Quick start, Streaming, Verify the connection (+1 more)

### Community 493 - "Red-Team Dispatch"
Cohesion: 0.20
Nodes (10): API, Configuration, Hardening loop, Job lifecycle, Ownership boundary, Red-Team Dispatch, Request flow, Runner contract (+2 more)

### Community 494 - "@trustloopguard/sdk"
Cohesion: 0.20
Nodes (9): Custom handlers, Gateway mode, Guard modes, Installation, License, Low-level client, Quick start, Requirements (+1 more)

### Community 495 - "layout.tsx"
Cohesion: 0.22
Nodes (7): ibmPlexMono, inter, metadata, RootLayoutProps, ThemeProvider(), Toaster(), sonner

### Community 496 - "scripts"
Cohesion: 0.22
Nodes (9): scripts, build, db:seed, dev, start, test, test:coverage, test:watch (+1 more)

### Community 497 - "SDK publishing"
Cohesion: 0.22
Nodes (6): Before tagging, Common failures, Publish, Release contract, SDK publishing, Verify

### Community 499 - "create_my_workspace"
Cohesion: 0.12
Nodes (31): AddMemberOutcome, create_invite(), create_my_workspace(), list_invites(), list_members(), list_my_workspaces(), revoke_invite(), Extension (+23 more)

### Community 500 - "api-keys.ts"
Cohesion: 0.16
Nodes (9): apiKeyBatchRevokeResponseSchema, apiKeySchema, revokeApiKeys(), ApiKeyBatchRevokeRequest, ApiKeyBatchRevokeResponse, ApiKeyListResponse, CreateApiKeyResponse, DashboardApiKey (+1 more)

### Community 501 - "JsonSchema"
Cohesion: 0.13
Nodes (24): Duration, Result, JsonSchema, LlmError, LlmOutput, Duration, String, Value (+16 more)

### Community 502 - "proxy_healthcare_agent.py"
Cohesion: 0.31
Nodes (7): entrypoint(), gateway_api_key(), gateway_openai_base_url(), HealthcareProxyAgent, livekit_run_external_id(), JobContext, LiveKit healthcare agent that routes its LLM through TrustLoopGuard gateway.  Th

### Community 503 - ".submit_event"
Cohesion: 0.31
Nodes (6): Client, Decision, GuardEvent, Option, Result, SdkError

### Community 504 - "header_value"
Cohesion: 0.25
Nodes (8): header_value(), log_http_response(), HeaderMap, Next, Option, Request, Response, String

### Community 505 - "marketing-event-link.tsx"
Cohesion: 0.21
Nodes (13): Footer(), getFooterEvent(), LINK_GROUPS, Status, MarketingEventLink(), MarketingEventLinkProps, mergeRel(), Status (+5 more)

### Community 506 - "mod.rs"
Cohesion: 0.36
Nodes (8): authority_template_substitutes_all_placeholders(), build(), hallucination_template_substitutes_all_placeholders(), String, schema(), schemas_have_required_fields(), semantic_policy_template_substitutes_all_placeholders(), tone_template_substitutes_all_placeholders()

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

### Community 512 - "KnowledgeStoreError"
Cohesion: 0.38
Nodes (10): KnowledgeStoreError, CreateKnowledgeSourceRequest, String, decode_file_data(), CreateKnowledgeSourceRequest, Result, Vec, validate_create_request() (+2 more)

### Community 513 - "PolicyEditorDialog.test.tsx"
Cohesion: 0.25
Nodes (6): generatePolicyDraft, getPolicy, NON_ROUNDTRIP_YAML, ROUNDTRIP_YAML, upsertPolicy, validatePolicy

### Community 515 - "auth_user.rs"
Cohesion: 0.20
Nodes (12): AuthUserState, normalize_oauth_provider(), oauth_session(), Json, Response, Arc, Option, Result (+4 more)

### Community 516 - "ApiError"
Cohesion: 0.15
Nodes (10): ApiError, ApiErrorCode, ApiErrorCode, Display, Formatter, Result, Self, String (+2 more)

### Community 517 - "patch_gateway_route"
Cohesion: 0.28
Nodes (12): create_gateway_route(), list_gateway_routes(), patch_gateway_route(), CreateGatewayRouteRequest, Extension, HeaderMap, Json, Option (+4 more)

### Community 518 - "runs_integration.rs"
Cohesion: 0.46
Nodes (7): event_body(), one_shot_retry(), RetryConfig, Value, run_body(), run_helpers_encode_ids_and_parse_typed_responses(), start_run_posts_typed_request_with_bearer_auth()

### Community 519 - "ProvenanceMap"
Cohesion: 0.36
Nodes (5): ProvenanceMap, BTreeMap, Into, String, Vec

### Community 520 - "fresh_repo"
Cohesion: 0.39
Nodes (7): fresh_repo(), insert_then_mark_failed(), insert_then_mark_sent(), list_stale_returns_only_old_pending(), record_attempt_increments_counter(), ContainerAsync, PostgresImage

### Community 521 - "ToolMetadataProvider"
Cohesion: 0.32
Nodes (9): FailingToolMetadataProvider, NoOpToolMetadataProvider, HashMap, Option, Result, ToolMetadata, StubToolMetadataProvider, ToolMetadataProvider (+1 more)

### Community 522 - "route.ts"
Cohesion: 0.16
Nodes (11): cleanupAgent(), createAgentSchema, GET(), POST(), stringListSchema, AgentClient, AgentProfileWire, mockState (+3 more)

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
Cohesion: 0.08
Nodes (36): HandlerCtx, Default, Self, aggregate(), DefaultTierRunner, Arc, CancellationToken, Decision (+28 more)

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

### Community 549 - ".create_financial_policy"
Cohesion: 0.24
Nodes (9): enforcing_action(), financial_policy_from_request(), financial_policy_record(), policy_action(), Action, CreateFinancialPolicyRequest, FinancialPolicyListResponse, FinancialPolicyRecord (+1 more)

### Community 550 - "devDependencies"
Cohesion: 0.29
Nodes (7): devDependencies, lefthook, prettier, secretlint, @secretlint/secretlint-rule-preset-recommend, tsx, yaml

### Community 552 - "analytics_query.rs"
Cohesion: 0.50
Nodes (3): HumanReviewAnalyticsFilter, Option, String

### Community 553 - "Verdict"
Cohesion: 0.50
Nodes (4): Verdict, description, enum, type

### Community 556 - "RunnerAttackSession"
Cohesion: 0.40
Nodes (5): RunnerAttackSession, additionalProperties, description, required, type

### Community 557 - "source_chain"
Cohesion: 0.50
Nodes (4): type, source_chain, items, type

### Community 558 - "properties"
Cohesion: 0.14
Nodes (14): items, type, ParamSpec, anyOf, description, properties, required, type (+6 more)

### Community 560 - "GatewayState"
Cohesion: 0.32
Nodes (11): GatewayState, proxy_anthropic_messages(), proxy_openai_chat_completions(), Bytes, Extension, HeaderMap, Option, Path (+3 more)

### Community 561 - ".list_policies"
Cohesion: 0.31
Nodes (7): Client, Option, PolicyDocument, PolicyFamily, PolicyListResponse, Result, SdkError

### Community 562 - "docs"
Cohesion: 0.33
Nodes (5): Content, Develop, docs, Password protection, Why a separate app

### Community 564 - "KnowledgeSourceDocument.ts"
Cohesion: 0.22
Nodes (7): CreateKnowledgeSourceRequest, DashboardKnowledgeSourceKind, KnowledgeFileInput, KnowledgeSourceDocument, RFC-3339, KnowledgeSourceListResponse, KnowledgeSourceStatus

### Community 565 - "package.json"
Cohesion: 0.33
Nodes (5): license, name, private, type, version

### Community 567 - "service.rs"
Cohesion: 0.08
Nodes (47): financial_matches(), FinancialLedgerEntryKind, FinancialStore, Send, Sync, action_decision(), authorization_scope_summary(), compose_policy_decisions() (+39 more)

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

### Community 578 - "defaults.rs"
Cohesion: 0.33
Nodes (5): default_views(), empty_catalog(), AnalyticsDashboardView, AnalyticsFacetCatalogResponse, Vec

### Community 579 - "proxy.ts"
Cohesion: 0.43
Nodes (7): config, isAuthenticated(), isPublicPath(), proxy(), PUBLIC_PATH_PREFIXES, safeRedirect(), SESSION_COOKIE_NAMES

### Community 580 - "api_error_response"
Cohesion: 0.24
Nodes (10): ai_edit_policy(), Bytes, Response, api_error_response(), api_error_response_with_details(), ApiErrorCode, Response, StatusCode (+2 more)

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

### Community 586 - "index.mdx"
Cohesion: 0.40
Nodes (4): CLI, HTTP API, Rust Crates, SDKs

### Community 587 - "gateway_routes"
Cohesion: 0.50
Nodes (4): build_gateway_http_client(), gateway_routes(), Client, Router

### Community 588 - "LlmUsageResponse.ts"
Cohesion: 0.23
Nodes (7): LlmUsageBucket, RFC-3339, LlmUsageBucketsResponse, LlmUsageEvent, RFC-3339, LlmUsageListResponse, LlmUsageResponse

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

### Community 594 - "finalize_gateway_response"
Cohesion: 0.36
Nodes (10): apply_enforcement_headers(), EnforcementHeaders, finalize_gateway_response(), handle_provider_failure(), EnforcementProfile, Option, P, Response (+2 more)

### Community 595 - "insert_trace"
Cohesion: 0.29
Nodes (10): analytics_distinguishes_guardrail_and_human_interventions(), fresh_pool(), insert_trace(), review_events_are_append_only_and_latest_is_queryable(), ContainerAsync, DbPool, Option, PostgresImage (+2 more)

### Community 596 - "The three rules"
Cohesion: 0.50
Nodes (4): 1. Engine-only PRs aren't done, 2. No internal imports in `demo/`, 3. Cross-cutting concerns live in the SDK, once, The three rules

### Community 597 - "view_from_record"
Cohesion: 0.27
Nodes (9): NewViewRecord, AnalyticsDashboardView, DateTime, Result, String, Utc, Value, view_from_record() (+1 more)

### Community 598 - "llm_usage.rs"
Cohesion: 0.39
Nodes (8): LlmUsageBucket, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageListResponse, LlmUsageResponse, String, Value, Vec

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

### Community 615 - "auth.rs"
Cohesion: 0.48
Nodes (6): AuthRequest, AuthResponse, ChangePasswordRequest, OAuthIdentityRequest, Option, String

### Community 618 - "validate_create_event"
Cohesion: 0.25
Nodes (8): clean_string(), normalize_metadata(), CreateHumanReviewEventRequest, Option, Result, String, Value, validate_create_event()

### Community 619 - "submit_event"
Cohesion: 0.36
Nodes (7): GuardEvent, HeaderMap, Json, Response, String, submit_event(), workspace_id_for_event()

### Community 665 - "api_error"
Cohesion: 0.39
Nodes (7): api_error(), invalid_credentials(), password_auth_disabled(), ApiErrorCode, Response, StatusCode, String

### Community 669 - "proxy_provider_request"
Cohesion: 0.12
Nodes (26): proxy_provider_request(), parse_provider_request(), prepare_streaming_request(), Bytes, EnforcementProfile, P, Response, Result (+18 more)

### Community 670 - "monitoring_sessions.rs"
Cohesion: 0.19
Nodes (16): event_body(), event_rejects_oversized_session_id(), event_trace_write_carries_session_id(), oversized_session(), post_json(), read_body(), RecordingTraceStore, Body (+8 more)

### Community 671 - "properties"
Cohesion: 0.15
Nodes (13): KnowledgeSource, type, type, allOf, default, properties, required, type (+5 more)

### Community 673 - "HumanReviewStoreError"
Cohesion: 0.29
Nodes (7): HumanReviewAnalyticsFilter, HumanReviewStoreError, review_error_response(), Response, Option, String, human_review_store_error()

### Community 674 - "llm_pricing.rs"
Cohesion: 0.38
Nodes (6): LlmModelPrice, LlmPriceSource, LlmPricingListResponse, String, Vec, UpsertLlmModelPriceRequest

### Community 676 - "fresh_pool"
Cohesion: 0.38
Nodes (6): event(), fresh_pool(), insert_window_sum_and_grouping_round_trip(), ContainerAsync, DbPool, PostgresImage

### Community 677 - "auth-redirect.ts"
Cohesion: 0.53
Nodes (4): AuthRedirectConfig, isRustOrLocalOrigin(), safeAuthRedirect(), config

### Community 678 - "hash_password"
Cohesion: 0.60
Nodes (5): hash_password(), PasswordError, Result, String, verify_password()

### Community 915 - "fresh_pool"
Cohesion: 0.40
Nodes (5): fresh_pool(), ContainerAsync, DbPool, PostgresImage, upsert_get_list_and_delete_round_trip()

### Community 1138 - "LlmModelPrice.ts"
Cohesion: 0.47
Nodes (3): LlmModelPrice, LlmPriceSource, LlmPricingListResponse

### Community 1581 - "index.mdx"
Cohesion: 0.40
Nodes (4): Core Ideas, Latency Model, Runtime Shape, Source Of Truth

### Community 1774 - "setup.ts"
Cohesion: 0.70
Nodes (4): enforceModes(), headers(), main(), tools

### Community 1803 - "trivial_schema"
Cohesion: 0.83
Nodes (3): openai_round_trip(), openrouter_round_trip(), trivial_schema()

### Community 1804 - "Verdict"
Cohesion: 0.50
Nodes (4): Verdict, description, enum, type

### Community 1807 - "Red-team harden (policy synthesis)"
Cohesion: 0.29
Nodes (7): Inputs and outputs, Outcome model, Ownership, Reachable substrates, Red-team harden (policy synthesis), What it does, Where it sits

### Community 1813 - "Agent Breakaway Arena"
Cohesion: 0.33
Nodes (6): Adapter Contract, Agent Breakaway Arena, Flow, Hardening Loop, Ownership Boundary, What The Agent Receives

## Knowledge Gaps
- **2789 isolated node(s):** `printWidth`, `tabWidth`, `useTabs`, `semi`, `singleQuote` (+2784 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **646 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `StorageError` connect `StorageError` to `PostgresAnalyticsAdapter`, `Result`, `PostgresRedteamJobAdapter`, `redteam.rs`, `EnvironmentRepo`, `Result`, `PostgresStore`, `RedteamPlanRepo`, `HumanReviewStoreError`, `PostgresUserAdapter`, `api_keys.rs`, `Result`, `latest_review_outcomes`, `insert_existing_workspace_member`, `PostgresGatewayAdapter`, `analytics_query.rs`, `llm_pricing.rs`, `ToolMetadataRepo`, `latest_review_outcomes`, `FinancialStoreError`, `.create_event`, `validation.rs`, `MemoryToolMetadataStore`, `LlmUsageRepo`, `.create_event`, `LlmUsageStoreError`, `gateway_repo.rs`, `LlmPricingRepo`, `PostgresLabelPolicyAdapter`, `profile_record_to_wire`, `EnvironmentStoreError`, `lib.rs`, `PolicyRepo`, `RunStoreError`, `models.rs`, `analytics.rs`, `AgentRepo`, `view_from_record`, `schema.rs`, `share.rs`, `metrics.rs`, `RunState`, `EscalationRepo`, `writer.rs`, `RedteamReportShareRepo`, `TeamStoreError`, `KnowledgeRepo`, `dashboard_admin_repo.rs`, `DashboardAdminStoreError`?**
  _High betweenness centrality (0.054) - this node is a cross-community bridge._
- **Why does `State` connect `State` to `create_knowledge_source`, `auth_user.rs`, `patch_gateway_route`, `put_llm_price`, `oauth.rs`, `api_error_response`, `plan.rs`, `harden-job-card.tsx`, `traces.rs`, `tests.rs`, `GatewayState`, `MemoryToolMetadataStore`, `api_error_response`, `generate_guardrails`, `PolicyState`, `llm_usage.rs`, `create_review_event`, `AuthConfig`, `lib.rs`, `change_password`, `RedteamState`, `patch_enforcement_profile`, `WorkspaceKeyContext`, `RunState`, `submit_event`, `create_my_workspace`?**
  _High betweenness centrality (0.053) - this node is a cross-community bridge._
- **Why does `AppState` connect `AppState` to `oauth.rs`, `traces.rs`, `router`, `Engine`, `proxy_provider_request`, `HandlerCtx`, `event_service.rs`, `GatewayState`, `service.rs`, `JwtSigner`, `mod.rs`, `gateway_routes`, `check_and_maybe_regenerate`, `financial_actions.rs`, `budget.rs`, `RedteamJobStore`, `patch_enforcement_profile`, `effective_checker_modes`, `share.rs`, `EventPipelineCtx`, `submit_event`, `event_ingestion.rs`, `spawn_escalation_worker`, `DashboardAdminStoreError`?**
  _High betweenness centrality (0.046) - this node is a cross-community bridge._
- **Are the 90 inferred relationships involving `Client` (e.g. with `Decode` and `SdkError`) actually correct?**
  _`Client` has 90 INFERRED edges - model-reasoned connections that need verification._
- **What connects `printWidth`, `tabWidth`, `useTabs` to the rest of the system?**
  _2864 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Client` be split into smaller, more focused modules?**
  _Cohesion score 0.06663141195134849 - nodes in this community are weakly interconnected._
- **Should `GuardEvent` be split into smaller, more focused modules?**
  _Cohesion score 0.12105263157894737 - nodes in this community are weakly interconnected._